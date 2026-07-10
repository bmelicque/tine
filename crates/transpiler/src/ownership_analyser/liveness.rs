// First pass of the ownership checker.
//
// Produces a `UseSites` map: for every variable (identified by its
// `SymbolId`), the list of `Location`s where that variable is read, with some
// context information relevant for the ownership analysis.
//
// Being captured by a loop or a closure effectively affects the variable's
// lifespan.

use std::collections::HashMap;

use tine_common::locations::Location;
use tine_ir as ir;
use tine_symbols::{symbols::SymbolId, table::SymbolTable};

/// A single use of a variable, with relevant context information.
#[derive(Debug, Default, Clone, Copy)]
pub struct UseSite {
    pub loc: Location,
    /// `true` when this use-site is a capture inside a loop body (For/ForIn).
    /// This use-site is counted like several use-sites (in case the loop body
    /// is repeatedly executed).
    pub in_loop: bool,
    /// `true` when this use-site is a capture inside a FunctionExpression.
    /// This use-site is counted like several use-sites (in case the closure
    /// is repeatedly executed).
    /// This use-site also counts as a delayed use-site, possibly far in the
    /// future.
    ///
    /// TODO: When async/await is implemented, this should also represent it.
    pub in_closure: bool,
    /// `true` if it is a function param or imported variable. Since we don't
    /// know where it's coming from, it could possible live forever.
    pub is_external: bool,
    /// `true` if this is the lhs of an assignment. This is relevant only for
    /// mutable variables.
    pub is_mutated: bool,
}

impl UseSite {
    /// A fresh use-site at the declaration of a variable.
    fn declaration(loc: Location) -> Self {
        Self {
            loc,
            ..Default::default()
        }
    }
}

/// The result of the analysis.
///
/// Contains all the `UseSite`s for any given variable.
#[derive(Debug, Default)]
pub struct UseSites(pub HashMap<SymbolId, Vec<UseSite>>);

impl UseSites {
    /// All use-sites of `sym` that occur strictly *after* `loc`.
    ///
    /// This also include the use-site at given location if it is captured by
    /// a loop (since the loop's body could be executed several times).
    pub fn uses_after(&self, sym: SymbolId, loc: Location) -> &[UseSite] {
        let Some(sites) = self.0.get(&sym) else {
            return &[];
        };
        let pos = sites
            .iter()
            .position(|s| s.loc == loc)
            .unwrap_or(sites.len());
        let skip: usize = sites.get(pos).map_or(0, |s| if s.in_loop { 0 } else { 1 });
        &sites[pos + skip..]
    }

    /// Check if the variable has escaped *before* its given use-site.
    pub fn has_escaped(&self, id: &ir::Identifier) -> bool {
        let Some(sites) = self.0.get(&id.symbol) else {
            // For safety, assume worst case (though this shouldn't happen and
            // probably be a panic)
            return true;
        };
        sites
            .iter()
            .take_while(|s| s.loc != id.loc)
            .find(|s| s.in_closure)
            .is_some()
    }

    pub fn is_external(&self, sym: &SymbolId) -> bool {
        self.0
            .get(sym)
            .unwrap_or(&vec![])
            .iter()
            .find(|s| s.is_external)
            .is_some()
    }

    /// True if `sym` is captured by a closure anywhere.
    pub fn captured_by_closure(&self, sym: &SymbolId) -> bool {
        self.0
            .get(sym)
            .map(|sites| sites.iter().any(|s| s.in_closure))
            .unwrap_or(false)
    }

    pub fn find(&self, id: &ir::Identifier) -> &UseSite {
        self.0
            .get(&id.symbol)
            .unwrap()
            .iter()
            .find(|s| s.loc == id.loc)
            .unwrap()
    }
}

/// The context threaded through the traversal.
/// Keeps track of current loop and closure bodies.
#[derive(Clone, Copy)]
struct Ctx<'sym> {
    loop_ctx: Option<Location>,
    closure_ctx: Option<Location>,
    in_param: bool,
    assignment_lhs: bool,
    in_callee: bool,
    symbols: &'sym SymbolTable,
}

impl Ctx<'_> {
    fn new<'sym>(symbols: &'sym SymbolTable) -> Ctx<'sym> {
        Ctx {
            loop_ctx: None,
            closure_ctx: None,
            in_param: false,
            assignment_lhs: false,
            in_callee: false,
            symbols,
        }
    }

    fn enter_loop(self, loc: Location) -> Self {
        Self {
            loop_ctx: Some(loc),
            ..self
        }
    }
    fn enter_closure(self, loc: Location) -> Self {
        Self {
            closure_ctx: Some(loc),
            ..self
        }
    }
    fn enter_assignment_lhs(self) -> Self {
        Self {
            assignment_lhs: true,
            ..self
        }
    }
    fn enter_callee(self) -> Self {
        Self {
            in_callee: true,
            ..self
        }
    }

    fn loop_captures(&self, sym: SymbolId) -> bool {
        let defined_at = self.symbols.get_symbol(sym).defined_at();
        self.loop_ctx
            .map(|ctx| defined_at.is_within(ctx))
            .unwrap_or(false)
    }
    fn closure_captures(&self, sym: SymbolId) -> bool {
        let defined_at = self.symbols.get_symbol(sym).defined_at();
        self.closure_ctx
            .map(|ctx| defined_at.is_within(ctx))
            .unwrap_or(false)
    }
}

/// Analyse the liveness of all symbols in the given program and returns a map
/// of their use-sites.
pub fn analyse_liveness(program: &ir::Program, symbols: &SymbolTable) -> UseSites {
    let mut sites = UseSites::default();
    let ctx = Ctx::new(symbols);
    for stmt in &program.statements {
        visit_stmt(stmt, ctx, &mut sites);
    }
    sites
}

fn visit_block(block: &ir::Block, ctx: Ctx, out: &mut UseSites) {
    for stmt in &block.statements {
        visit_stmt(stmt, ctx, out);
    }
}

fn visit_stmt(stmt: &ir::Statement, ctx: Ctx, out: &mut UseSites) {
    match stmt {
        ir::Statement::Variable(v) => visit_expr(&v.value, ctx, out),
        ir::Statement::Assignment(a) => {
            visit_expr(&a.value, ctx, out);
            // The pattern side (e.g. `a.field`) can itself reference variables.
            // Since the rhs is resolved before the actual assignment, this is
            // checked last.
            visit_expr(&a.pattern, ctx.enter_assignment_lhs(), out);
        }
        ir::Statement::Expression(e) => visit_expr(e, ctx, out),
        ir::Statement::Return(r) => {
            if let Some(e) = &r.expression {
                visit_expr(e, ctx, out);
            }
        }
        ir::Statement::Break(b) => {
            if let Some(e) = &b.expression {
                visit_expr(e, ctx, out);
            }
        }
        ir::Statement::Function(f) => {
            for param in &f.params {
                out.0.entry(param.1.into()).or_default().push(UseSite {
                    loc: param.0,
                    in_loop: false,
                    in_closure: false,
                    is_external: true,
                    is_mutated: false,
                })
            }
            visit_block(&f.body, ctx.enter_closure(f.body.loc), out);
        }
        ir::Statement::Method(m) => {
            out.0.insert(
                m.receiver_type.1.into(),
                vec![UseSite {
                    loc: m.receiver_type.0,
                    is_external: true,
                    ..Default::default()
                }],
            );
            for param in &m.params {
                out.0.insert(
                    param.1.into(),
                    vec![UseSite {
                        loc: param.0,
                        is_external: true,
                        ..Default::default()
                    }],
                );
            }
            visit_block(&m.body, ctx.enter_closure(m.body.loc), out);
        }

        // Type-level declarations carry no runtime use-sites.
        ir::Statement::Enum(_)
        | ir::Statement::Struct(_)
        | ir::Statement::Use(_)
        | ir::Statement::Continue(_) => {}
    }
}

fn visit_expr(expr: &ir::Expression, ctx: Ctx, out: &mut UseSites) {
    match expr {
        // Leaf: record the use-site
        ir::Expression::Identifier(id) => {
            let in_loop = ctx.loop_captures(id.symbol);
            let in_closure = ctx.closure_captures(id.symbol);
            out.0.entry(id.symbol.clone()).or_default().push(UseSite {
                loc: id.loc,
                in_loop,
                in_closure,
                is_external: ctx.in_param,
                is_mutated: ctx.assignment_lhs || ctx.in_callee,
            });
        }

        ir::Expression::BooleanLiteral(_)
        | ir::Expression::FloatLiteral(_)
        | ir::Expression::IntLiteral(_)
        | ir::Expression::StringLiteral(_) => {}

        ir::Expression::Unary(u) => visit_expr(&u.operand, ctx, out),
        ir::Expression::Member(m) => visit_expr(&m.object, ctx, out),
        ir::Expression::Method(m) => visit_expr(&m.host, ctx, out),
        ir::Expression::Binary(b) => {
            visit_expr(&b.left, ctx, out);
            visit_expr(&b.right, ctx, out);
        }
        ir::Expression::Block(b) => visit_block(b, ctx, out),

        ir::Expression::Array(a) => {
            for e in &a.elements {
                visit_expr(e, ctx, out);
            }
        }
        ir::Expression::Tuple(t) => {
            for e in &t.elements {
                visit_expr(e, ctx, out);
            }
        }
        ir::Expression::Map(m) => {
            for entry in &m.entries {
                visit_expr(&entry.key, ctx, out);
                visit_expr(&entry.value, ctx, out);
            }
        }
        ir::Expression::Struct(s) => {
            for field in &s.fields {
                visit_expr(&field.value, ctx, out);
            }
        }

        ir::Expression::Call(c) => {
            visit_expr(&c.callee, ctx.enter_callee(), out);
            for arg in &c.args {
                visit_expr(arg, ctx, out);
            }
        }

        ir::Expression::If(ir::IfExpression {
            condition,
            consequent,
            alternate,
            ..
        }) => {
            visit_expr(condition, ctx, out);
            visit_block(consequent, ctx, out);
            if let Some(alt) = alternate {
                visit_block(alt, ctx, out);
            }
        }

        ir::Expression::For(ir::ForExpression {
            condition,
            body,
            loc,
            ..
        }) => {
            if let Some(cond) = condition {
                visit_expr(cond, ctx.enter_loop(*loc), out);
            }
            visit_block(body, ctx.enter_loop(*loc), out);
        }
        ir::Expression::ForIn(ir::ForInExpression {
            iterable,
            element,
            body,
            loc,
            ..
        }) => {
            // The iterable is evaluated once, outside the loop.
            visit_expr(iterable, ctx, out);
            // The element binding is freshly introduced each iteration;
            out.0
                .entry(element.1.into())
                .or_default()
                .push(UseSite::declaration(element.0));
            visit_block(body, ctx.enter_loop(*loc), out);
        }

        ir::Expression::Function(ir::FunctionExpression {
            body, loc, params, ..
        }) => {
            for param in params {
                out.0.entry(param.1.into()).or_default().push(UseSite {
                    loc: param.0,
                    is_external: true,
                    ..Default::default()
                })
            }
            visit_block(body, ctx.enter_closure(*loc), out);
        }

        ir::Expression::Element(e) => {
            for attr in &e.attributes {
                visit_expr(&attr.value, ctx, out);
            }
            for child in &e.children {
                visit_expr(child, ctx, out);
            }
        }

        ir::Expression::TypeMatch(t) => visit_expr(&t.expr, ctx, out),
    }
}
