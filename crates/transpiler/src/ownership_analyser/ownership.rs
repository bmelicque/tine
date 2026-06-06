// Last pass of the ownership checker.
//
// Consumes the `UseSites` map produced by the liveness pass and the `AliasMap`
// produced by the alias pass, and annotates every `ir::Identifier` use with an
// `OwnershipAction`:
//
// | Action  | JS output    | Condition                                       |
// |---------|--------------|-------------------------------------------------|
// | Borrow  | `x`          | No mutation possible, alias group still live    |
// | Move    | `x`          | Entire alias group dead after this point        |
// | Clone   | `x.$clone()` | Any other case                                  |
//

use std::collections::HashMap;

use tine_core::{ir, Location, Session};

use super::alias::AliasMap;
use super::liveness::UseSites;
use super::semantics::SemanticsChecker;

/// The action used by the codegenerator on a use-site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnershipAction {
    /// Copy: for JS primitives, functions and objects that would still be
    /// handled by reference.
    Copy,
    /// Borrow: pass the JS reference as-is.
    Borrow,
    /// Last use of the entire alias group. Pass the JS reference and
    /// invalidate the binding. No clone needed.
    Move,
    /// Alias group still live with potential mutation. Should be deep-cloned.
    Clone,
}

/// Maps each use-site location to the action the codegen should take.
///
/// The key is the `Location` of the `ir::Identifier` node, which is unique
/// per use-site.
#[derive(Debug, Default)]
pub struct OwnershipMap(pub HashMap<Location, OwnershipAction>);

impl OwnershipMap {
    /// Look up the action for a given identifier use.
    /// Falls back to `Clone` (the safe default) if the location was not
    /// annotated (this should not happen in correct usage).
    pub fn action_for(&self, loc: Location) -> OwnershipAction {
        self.0.get(&loc).copied().unwrap_or(OwnershipAction::Clone)
    }
}

/// Describes the binding context of the current expression, ie what the
/// current expression will be bound to.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Binding {
    /// No binding context: an expression whose result is fresh.
    #[default]
    None,
    /// Immutable variable declaration or call arguments whose borrows do not
    /// outlive the expression.
    Immutable(Location),
    /// Mutated variable declaration or assigned value.
    Mutable(Location),
    /// For call arguments, if the borrow does not outlive the call.
    Any(Location),
}

impl Binding {
    fn mutable(loc: Location) -> Self {
        Self::Mutable(loc)
    }
}

#[derive(Default, Clone, Copy)]
struct Ctx {
    forced_copy: bool,
    binding: Binding,
    /// The binding kind `break` statements should refer to. This is what the
    /// loop's value will be assigned to.
    loop_binding: Binding,
}

impl Ctx {
    fn root() -> Self {
        Self::default()
    }

    fn force_copying(&self) -> Self {
        Self {
            forced_copy: true,
            ..*self
        }
    }

    fn enter_binding(&self, binding: Binding) -> Self {
        Self { binding, ..*self }
    }
    fn enter_non_binding(&self) -> Self {
        Self {
            binding: Binding::None,
            ..*self
        }
    }

    fn enter_loop(&self, binding: Binding) -> Self {
        Self {
            loop_binding: binding,
            ..*self
        }
    }

    fn in_expression(&self) -> bool {
        self.binding == Binding::None
    }
}

/// Analyse the program and determine the ownership of each variable.
pub fn analyse_ownership(
    program: &ir::Program,
    sites: &UseSites,
    aliases: &mut AliasMap,
    session: &Session,
) -> OwnershipMap {
    let mut map = OwnershipMap::default();
    let ctx = Ctx::root();
    let semantics = SemanticsChecker::new(session);
    for stmt in &program.statements {
        visit_stmt(stmt, ctx, sites, aliases, &semantics, &mut map);
    }
    map
}

/// Decide the `OwnershipAction` for a single identifier use.
fn resolve(
    id: &ir::Identifier,
    ctx: Ctx,
    sites: &UseSites,
    aliases: &mut AliasMap,
) -> OwnershipAction {
    if ctx.in_expression() {
        return OwnershipAction::Borrow;
    }

    if id.symbol.borrow().is_mutable() {
        resolve_mutable(id, ctx, sites)
    } else {
        resolve_immutable(id, ctx, sites, aliases)
    }
}

/// Decide the `OwnershipAction` for a single immutable symbol use.
fn resolve_immutable(
    id: &ir::Identifier,
    ctx: Ctx,
    sites: &UseSites,
    aliases: &mut AliasMap,
) -> OwnershipAction {
    let defined_at = id.symbol.borrow().defined_at;
    match ctx.binding {
        Binding::None | Binding::Immutable(_) | Binding::Any(_) => return OwnershipAction::Borrow,
        Binding::Mutable(loc) if defined_at.is_within(loc) => return OwnershipAction::Borrow,
        Binding::Mutable(_) => {}
    }

    let escapes = aliases
        .group_members(&id.symbol)
        .iter()
        .any(|sym| sites.is_external(sym) || sites.captured_by_closure(sym));
    if escapes {
        return OwnershipAction::Clone;
    }

    let group_live = aliases.group_live_after(id, sites);
    if !group_live {
        return OwnershipAction::Move;
    }

    return OwnershipAction::Clone;
}

/// Decide the `OwnershipAction` for a single mutable symbol use.
fn resolve_mutable(id: &ir::Identifier, ctx: Ctx, sites: &UseSites) -> OwnershipAction {
    if matches!(ctx.binding, Binding::None | Binding::Any(_)) {
        return OwnershipAction::Borrow;
    }

    if sites.has_escaped(id) {
        return OwnershipAction::Clone;
    }

    let uses_after = sites.uses_after(id.symbol.clone(), id.loc);
    match uses_after.first() {
        Some(next) if uses_after.len() == 1 && next.is_assignee => OwnershipAction::Move,
        Some(_) => OwnershipAction::Clone,
        None => OwnershipAction::Move,
    }
}

fn visit_block(
    block: &ir::Block,
    ctx: Ctx,
    sites: &UseSites,
    aliases: &mut AliasMap,
    semantics: &SemanticsChecker,
    out: &mut OwnershipMap,
) {
    for stmt in &block.statements {
        visit_stmt(stmt, ctx.clone(), sites, aliases, semantics, out);
    }
}

fn visit_block_expr(
    block: &ir::Block,
    ctx: Ctx,
    sites: &UseSites,
    aliases: &mut AliasMap,
    semantics: &SemanticsChecker,
    out: &mut OwnershipMap,
) {
    let stmt_ctx = ctx.enter_non_binding();
    block
        .statements
        .iter()
        .take(block.statements.len() - 1)
        .for_each(|stmt| visit_stmt(stmt, stmt_ctx, sites, aliases, semantics, out));
    match block.statements.last() {
        Some(ir::Statement::Expression(expr)) => {
            visit_expr(expr, ctx, sites, aliases, semantics, out);
        }
        Some(stmt) => {
            visit_stmt(stmt, stmt_ctx, sites, aliases, semantics, out);
        }
        None => {}
    }
}

fn visit_stmt(
    stmt: &ir::Statement,
    ctx: Ctx,
    sites: &UseSites,
    aliases: &mut AliasMap,
    semantics: &SemanticsChecker,
    out: &mut OwnershipMap,
) {
    match stmt {
        ir::Statement::Variable(v) => {
            let binding = if v.mutable {
                Binding::Mutable(v.loc)
            } else {
                Binding::Immutable(v.loc)
            };
            visit_expr(
                &v.value,
                ctx.enter_binding(binding),
                sites,
                aliases,
                semantics,
                out,
            )
        }
        ir::Statement::Assignment(a) => {
            visit_expr(&a.pattern, ctx, sites, aliases, semantics, out);
            visit_expr(
                &a.value,
                ctx.enter_binding(Binding::mutable(a.value.loc())),
                sites,
                aliases,
                semantics,
                out,
            );
        }
        ir::Statement::Expression(e) => visit_expr(e, ctx, sites, aliases, semantics, out),
        ir::Statement::Return(r) => {
            if let Some(e) = &r.expression {
                visit_expr(e, ctx.enter_non_binding(), sites, aliases, semantics, out);
            }
        }
        ir::Statement::Break(b) => {
            if let Some(e) = &b.expression {
                visit_expr(
                    e,
                    ctx.enter_binding(ctx.loop_binding),
                    sites,
                    aliases,
                    semantics,
                    out,
                );
            }
        }
        ir::Statement::Function(f) => visit_block(&f.body, ctx, sites, aliases, semantics, out),
        ir::Statement::Enum(_)
        | ir::Statement::Struct(_)
        | ir::Statement::Use(_)
        | ir::Statement::Continue(_) => {}
    }
}

fn visit_expr(
    expr: &ir::Expression,
    ctx: Ctx,
    sites: &UseSites,
    aliases: &mut AliasMap,
    semantics: &SemanticsChecker,
    out: &mut OwnershipMap,
) {
    match expr {
        // --- The core annotation site ---
        ir::Expression::Identifier(id) => {
            let action = if ctx.forced_copy || semantics.is_copy(id.ty()) {
                OwnershipAction::Copy
            } else {
                resolve(id, ctx, sites, aliases)
            };
            out.0.insert(id.loc, action);
        }
        ir::Expression::Member(m) => {
            // Force the expression's semantics onto root identifier.
            let ctx = if semantics.is_copy(m.ty) {
                ctx.force_copying()
            } else {
                ctx
            };
            visit_expr(&m.object, ctx, sites, aliases, semantics, out)
        }

        ir::Expression::BooleanLiteral(_)
        | ir::Expression::FloatLiteral(_)
        | ir::Expression::IntLiteral(_)
        | ir::Expression::StringLiteral(_) => {}

        ir::Expression::Unary(u) => match u.operator {
            // Behavior could change with new operators.
            ir::UnaryOperator::Bang | ir::UnaryOperator::Minus | ir::UnaryOperator::Star => {
                visit_expr(
                    &u.operand,
                    ctx.enter_non_binding(),
                    sites,
                    aliases,
                    semantics,
                    out,
                );
            }
        },
        ir::Expression::Binary(b) => match b.op {
            // Behavior could change with new operators.
            ir::BinaryOperator::Add
            | ir::BinaryOperator::Div
            | ir::BinaryOperator::EqEq
            | ir::BinaryOperator::Geq
            | ir::BinaryOperator::Grt
            | ir::BinaryOperator::LAnd
            | ir::BinaryOperator::Leq
            | ir::BinaryOperator::Less
            | ir::BinaryOperator::LOr
            | ir::BinaryOperator::Mod
            | ir::BinaryOperator::Mul
            | ir::BinaryOperator::Neq
            | ir::BinaryOperator::Pow
            | ir::BinaryOperator::Sub => {
                visit_expr(
                    &b.left,
                    ctx.enter_non_binding(),
                    sites,
                    aliases,
                    semantics,
                    out,
                );
                visit_expr(
                    &b.right,
                    ctx.enter_non_binding(),
                    sites,
                    aliases,
                    semantics,
                    out,
                );
            }
        },
        ir::Expression::TypeMatch(t) => {
            visit_expr(
                &t.expr,
                ctx.enter_non_binding(),
                sites,
                aliases,
                semantics,
                out,
            );
        }

        ir::Expression::Array(a) => {
            for e in &a.elements {
                visit_expr(e, ctx, sites, aliases, semantics, out);
            }
        }
        ir::Expression::Tuple(t) => {
            for e in &t.elements {
                visit_expr(e, ctx, sites, aliases, semantics, out);
            }
        }
        ir::Expression::Map(m) => {
            for entry in &m.entries {
                visit_expr(&entry.key, ctx, sites, aliases, semantics, out);
                visit_expr(&entry.value, ctx, sites, aliases, semantics, out);
            }
        }
        ir::Expression::Struct(s) => {
            for field in &s.fields {
                visit_expr(&field.value, ctx, sites, aliases, semantics, out);
            }
        }
        ir::Expression::Element(e) => {
            for attr in &e.attributes {
                visit_expr(&attr.value, ctx, sites, aliases, semantics, out);
            }
            for child in &e.children {
                visit_expr(child, ctx, sites, aliases, semantics, out);
            }
        }

        ir::Expression::Block(b) => visit_block_expr(b, ctx, sites, aliases, semantics, out),

        ir::Expression::Call(c) => {
            visit_expr(&c.callee, ctx, sites, aliases, semantics, out);
            let signature = aliases.alias_signature(&c.callee, semantics).cloned();

            for (i, arg) in c.args.iter().enumerate() {
                let is_aliased = signature.as_ref().map_or(true, |s| s.is_param_aliased(i));
                let binding = if is_aliased {
                    Binding::Immutable(arg.loc())
                } else {
                    Binding::Any(arg.loc())
                };
                visit_expr(
                    arg,
                    ctx.enter_binding(binding),
                    sites,
                    aliases,
                    semantics,
                    out,
                );
            }
        }

        ir::Expression::If(ir::IfExpression {
            condition,
            consequent,
            alternate,
            ..
        }) => {
            visit_expr(
                condition,
                ctx.enter_non_binding(),
                sites,
                aliases,
                semantics,
                out,
            );
            visit_block_expr(consequent, ctx, sites, aliases, semantics, out);
            if let Some(alt) = alternate {
                visit_block_expr(alt, ctx, sites, aliases, semantics, out);
            }
        }

        ir::Expression::For(ir::ForExpression {
            condition, body, ..
        }) => {
            if let Some(cond) = condition {
                visit_expr(cond, ctx.clone(), sites, aliases, semantics, out);
            }
            visit_block(
                body,
                ctx.enter_loop(ctx.binding),
                sites,
                aliases,
                semantics,
                out,
            );
        }

        ir::Expression::ForIn(ir::ForInExpression {
            iterable,
            element,
            body,
            ..
        }) => {
            visit_expr(iterable, ctx.clone(), sites, aliases, semantics, out);
            // The element binding is a fresh value each iteration: always
            // borrowed, never moved out of the loop.
            out.0.insert(element.loc, OwnershipAction::Borrow);
            visit_block(
                body,
                ctx.enter_loop(ctx.binding),
                sites,
                aliases,
                semantics,
                out,
            );
        }

        ir::Expression::Function(ir::FunctionExpression { body, .. }) => {
            visit_block(body, ctx, sites, aliases, semantics, out);
        }
    }
}
