// Second pass of the ownership checker.
//
// Builds a Union-Find structure that groups variables which *may* share the
// same underlying JS object. Two variables in the same group cannot be
// independently moved or mutably borrowed without affecting each other.
//
// Since aliases only apply to immutable variables, only `VariableDeclaration`s
// can introduce new aliases.
//
// Some expressions will borrow but produce a fresh value, thus not forwarding
// any alias.
//
// The ownership checker will not check liveness on single variables, but
// rather on their alias group.

use std::collections::HashMap;
use std::usize;

use tine_core::symbols::{SymbolId, SymbolTable, VariableSymbolId};
use tine_core::type_store::TypeStore;
use tine_core::{ir, Location};

use super::liveness::UseSites;
use super::semantics::SemanticsChecker;

/// A simple Union-Find (disjoint-set) over `SymbolId`s.
///
/// Each set represents a group of variables that may share the same JS object.
#[derive(Debug, Default)]
struct UnionFind {
    /// Maps each symbol to its canonical representative.
    /// If absent, the symbol is its own representative.
    parent: HashMap<SymbolId, SymbolId>,
}

impl UnionFind {
    /// Find the canonical representative of `sym`, with path compression.
    fn find(&mut self, sym: SymbolId) -> SymbolId {
        let Some(parent) = self.parent.get(&sym).cloned() else {
            return sym;
        };

        if parent == sym {
            return sym.clone();
        }
        let root = self.find(parent);
        self.parent.insert(sym.clone(), root.clone());
        root
    }

    /// Union the groups of `a` and `b`.
    fn union(&mut self, a: SymbolId, b: SymbolId) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra != rb {
            // Arbitrary choice: rb's root becomes ra's parent.
            self.parent.insert(rb, ra);
        }
    }
}

/// The alias signature of a function.
/// Resolves all aliasing between the function's params, captured variables and
/// returned values.
#[derive(Debug, Default, Clone)]
pub struct AliasSignature {
    /// Each of the function's param identified by position.
    /// `true` if aliases with the function's return value.
    params: Vec<bool>,
    /// All the constants captured by the functions that are intricated with
    /// the function's return value.
    captures: Vec<SymbolId>,
    /// `true` if the function is a method and its receiver might aliases the
    /// return value.
    is_receiver_aliased: bool,
}

impl AliasSignature {
    fn infer_from_function(
        def: &ir::FunctionExpression,
        return_group: Vec<SymbolId>,
        semantics: &SemanticsChecker,
    ) -> Self {
        let def_loc = def.loc;
        let params = get_param_aliases(&def.params, &return_group);
        let captures = get_captures(return_group, def_loc, semantics);

        Self {
            params,
            captures,
            is_receiver_aliased: false,
        }
    }

    fn infer_from_method(
        def: &ir::MethodDefinition,
        return_group: Vec<SymbolId>,
        semantics: &SemanticsChecker,
    ) -> Self {
        let def_loc = def.loc;
        let is_receiver_aliased = return_group
            .iter()
            .find(|id| **id == def.receiver_type.1.into())
            .is_some();
        let params = get_param_aliases(&def.params, &return_group);
        let captures = get_captures(return_group, def_loc, semantics);

        Self {
            params,
            captures,
            is_receiver_aliased,
        }
    }

    pub fn is_param_aliased(&self, pos: usize) -> bool {
        self.params[pos]
    }

    pub fn is_receiver_aliased(&self) -> bool {
        self.is_receiver_aliased
    }
}
fn get_param_aliases(
    params: &[(Location, VariableSymbolId)],
    return_group: &[SymbolId],
) -> Vec<bool> {
    params
        .iter()
        .map(|p| return_group.contains(&p.1.into()))
        .collect()
}
fn get_captures(
    return_group: Vec<SymbolId>,
    loc: Location,
    semantics: &SemanticsChecker,
) -> Vec<SymbolId> {
    return_group
        .into_iter()
        .filter(|s| !semantics.get_symbol(*s).defined_at().is_within(loc))
        .collect()
}

/// The result of alias analysis: a queryable map from symbol to alias group.
#[derive(Debug)]
pub struct AliasMap {
    /// This symbol represents unknown aliases, that could thus live forever.
    /// A symbol aliased with this can never be moved.
    forever: SymbolId,
    uf: UnionFind,
    /// All functions charts organized by definition location.
    signatures: HashMap<Location, AliasSignature>,
    /// All known symbols, so we can enumerate group members.
    all_symbols: Vec<SymbolId>,
}

impl AliasMap {
    /// Create a new `AliasMap`
    pub fn new() -> Self {
        Self {
            forever: SymbolId::dummy(),
            uf: UnionFind::default(),
            signatures: HashMap::new(),
            all_symbols: vec![],
        }
    }

    /// Return all symbols in the same alias group as `sym`.
    pub fn group_members(&mut self, sym: &SymbolId) -> Vec<SymbolId> {
        let root = self.uf.find(sym.clone());
        self.all_symbols
            .iter()
            .filter(|s| self.uf.find((*s).clone()) == root)
            .cloned()
            .collect()
    }

    /// Return `true` if any member of `sym`'s alias group has a use-site
    /// after `loc` in `sites`. Used by the ownership pass to decide whether
    /// a Move or BorrowMut is safe.
    pub fn group_live_after(&mut self, id: &ir::Identifier, sites: &UseSites) -> bool {
        self.group_members(&id.symbol)
            .into_iter()
            .any(|member| member == self.forever || !sites.uses_after(member, id.loc).is_empty())
    }

    fn register(&mut self, sym: SymbolId) {
        if !self.all_symbols.contains(&sym) {
            self.all_symbols.push(sym);
        }
    }

    fn union(&mut self, a: SymbolId, b: SymbolId) {
        self.register(a.clone());
        self.register(b.clone());
        self.uf.union(a, b);
    }

    fn union_all(&mut self, syms: &[SymbolId]) {
        if syms.len() < 2 {
            return;
        }
        let first = &syms[0];
        for sym in syms.iter().skip(1) {
            self.union(first.clone(), sym.clone());
        }
    }

    pub fn alias_signature<'map>(
        &'map self,
        callee: &ir::Expression,
        checker: &SemanticsChecker,
    ) -> Option<&'map AliasSignature> {
        match callee {
            ir::Expression::Identifier(i) => self
                .signatures
                .get(&checker.get_symbol(i.symbol).defined_at()),
            ir::Expression::Method(m) => {
                if checker.is_trait(m.host.ty()) {
                    return None;
                }
                eprintln!("Cannot perform function signature optimization on methods (not implemented yet). Defaulting to safe mode.");
                None
            }
            ir::Expression::Function(f) => {
                let loc = f.name.as_ref().map_or(f.loc, |n| n.0);
                self.signatures.get(&loc)
            }
            _ => None,
        }
    }
}

/// Analyse the program to determine which variables are aliased.
pub fn analyse_aliases(
    program: &ir::Program,
    types: &TypeStore,
    symbols: &SymbolTable,
) -> AliasMap {
    let mut map = AliasMap::new();
    let checker = SemanticsChecker::new(types, symbols);
    for stmt in &program.statements {
        visit_stmt(stmt, &checker, &mut map);
    }
    map
}

fn visit_block(block: &ir::Block, checker: &SemanticsChecker, map: &mut AliasMap) -> Vec<SymbolId> {
    block
        .statements
        .iter()
        .take(block.statements.len() - 1)
        .for_each(|stmt| visit_stmt(stmt, checker, map));
    block.statements.last().map_or(vec![], |stmt| match stmt {
        ir::Statement::Expression(e) => visit_expr(e, checker, map),
        _ => vec![],
    })
}

fn visit_stmt(stmt: &ir::Statement, checker: &SemanticsChecker, map: &mut AliasMap) {
    match stmt {
        ir::Statement::Variable(v) => {
            let roots = visit_expr(&v.value, checker, map);
            let ty = checker.get_symbol(v.symbol.into()).ty();
            if !v.mutable && !checker.is_copy(ty) {
                map.register(v.symbol.into());
                roots
                    .into_iter()
                    .filter(|root| !checker.is_mutable(*root))
                    .for_each(|root| map.union(v.symbol.into(), root));
            }
        }

        ir::Statement::Assignment(a) => {
            visit_expr(&a.value, checker, map);
        }

        ir::Statement::Expression(e) => {
            visit_expr(e, checker, map);
        }
        ir::Statement::Return(r) => {
            if let Some(e) = &r.expression {
                visit_expr(e, checker, map);
            }
        }
        ir::Statement::Break(b) => {
            if let Some(e) = &b.expression {
                visit_expr(e, checker, map);
            }
        }
        ir::Statement::Function(f) => {
            visit_function_expression(&f.clone().into(), checker, map);
        }
        ir::Statement::Method(m) => {
            visit_method(&m.clone().into(), checker, map);
        }
        ir::Statement::Enum(_)
        | ir::Statement::Struct(_)
        | ir::Statement::Use(_)
        | ir::Statement::Continue(_) => {}
    }
}

/// Gather the aliases captured by the expression, and forward them to the
/// parent expression if they could be represented by the same JS object.
fn visit_expr(
    expr: &ir::Expression,
    checker: &SemanticsChecker,
    map: &mut AliasMap,
) -> Vec<SymbolId> {
    match expr {
        ir::Expression::Identifier(id) => {
            let ty = checker.get_symbol(id.symbol.into()).ty();
            if checker.is_mutable(id.symbol) || checker.is_copy(ty) {
                return vec![];
            }
            map.register(id.symbol.into());
            return vec![id.symbol.into()];
        }
        ir::Expression::Member(m) => {
            if checker.is_copy(m.ty) {
                visit_expr(&m.object, checker, map);
                return vec![];
            }
            visit_expr(&m.object, checker, map)
        }
        ir::Expression::Method(m) => {
            visit_expr(&m.host, checker, map);
            vec![]
        }

        ir::Expression::BooleanLiteral(_)
        | ir::Expression::FloatLiteral(_)
        | ir::Expression::IntLiteral(_)
        | ir::Expression::StringLiteral(_) => vec![],

        // --- Non-forwarding expressions (produce a fresh value) ---
        ir::Expression::Unary(u) => match u.operator {
            // Behavior could change with new operators.
            ir::UnaryOperator::Bang | ir::UnaryOperator::Minus | ir::UnaryOperator::Star => {
                visit_expr(&u.operand, checker, map);
                vec![]
            }
            ir::UnaryOperator::Mut => unreachable!(),
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
                visit_expr(&b.left, checker, map);
                visit_expr(&b.right, checker, map);
                vec![]
            }
        },
        ir::Expression::TypeMatch(t) => {
            visit_expr(&t.expr, checker, map);
            vec![]
        }

        // --- These produce a new JS object that could be linked to an
        //     identifier used in the expression ---
        ir::Expression::Array(a) => a
            .elements
            .iter()
            .flat_map(|e| visit_expr(e, checker, map))
            .collect(),
        ir::Expression::Tuple(t) => t
            .elements
            .iter()
            .flat_map(|e| visit_expr(e, checker, map))
            .collect(),
        ir::Expression::Map(m) => m
            .entries
            .iter()
            // keys should be hashed so they can be ignored
            .flat_map(|e| visit_expr(&e.value, checker, map))
            .collect(),
        ir::Expression::Struct(s) => s
            .fields
            .iter()
            .flat_map(|f| visit_expr(&f.value, checker, map))
            .collect(),
        ir::Expression::Element(e) => e
            .attributes
            .iter()
            .map(|a| &a.value)
            .chain(&e.children)
            .flat_map(|e| visit_expr(e, checker, map))
            .collect(),

        ir::Expression::Block(b) => visit_block(b, checker, map),
        ir::Expression::Call(c) => {
            let chart = map.alias_signature(&c.callee, checker).cloned();
            match chart {
                Some(chart) => c
                    .args
                    .iter()
                    .map(|a| visit_expr(a, checker, map))
                    .zip(chart.params)
                    .filter(|(_, aliased)| *aliased)
                    .flat_map(|(aliases, _)| aliases)
                    .chain(chart.captures)
                    .collect::<Vec<_>>(),
                None => c
                    .args
                    .iter()
                    .flat_map(|a| visit_expr(a, checker, map))
                    .collect::<Vec<_>>(),
            }
        }
        ir::Expression::If(i) => {
            let mut cons = visit_block(&i.consequent, checker, map);
            let alt = i
                .alternate
                .as_ref()
                .map_or(vec![], |alt| visit_block(alt, checker, map));
            cons.extend(alt);
            cons
        }
        ir::Expression::For(f) => {
            if let Some(cond) = &f.condition {
                visit_expr(cond, checker, map);
            }
            visit_loop_body(&f.body, checker, map)
        }
        ir::Expression::ForIn(f) => {
            let iterable = visit_expr(&f.iterable, checker, map);

            let ty = checker.get_symbol(f.element.1.into()).ty();
            if !checker.is_copy(ty) {
                map.register(f.element.1.into());
                iterable
                    .iter()
                    .for_each(|sym| map.union(f.element.1.into(), *sym));
            }
            visit_loop_body(&f.body, checker, map)
        }
        ir::Expression::Function(f) => visit_function_expression(f, checker, map),
    }
}

fn visit_loop_body(
    body: &ir::Block,
    checker: &SemanticsChecker,
    map: &mut AliasMap,
) -> Vec<SymbolId> {
    visit_block(body, checker, map);
    let breaks = body
        .find_breaks()
        .into_iter()
        .filter_map(|r| r.expression.as_ref())
        .flat_map(|expr| visit_expr(&expr, checker, map))
        .collect::<Vec<_>>();
    map.union_all(&breaks);
    breaks
}

fn visit_function_expression(
    f: &ir::FunctionExpression,
    checker: &SemanticsChecker,
    map: &mut AliasMap,
) -> Vec<SymbolId> {
    f.params.iter().for_each(|p| map.register(p.1.into()));
    let mut returns = visit_block(&f.body, checker, map);
    f.body
        .find_returns()
        .into_iter()
        .filter_map(|r| r.expression.as_ref())
        .for_each(|expr| returns.extend(visit_expr(&expr, checker, map)));
    map.union_all(&returns);

    if returns.len() > 0 {
        let loc = f.name.as_ref().map_or(f.loc, |n| n.0);
        map.signatures.insert(
            loc,
            AliasSignature::infer_from_function(f, returns, checker),
        );
    }

    vec![]
}

fn visit_method(
    m: &ir::MethodDefinition,
    checker: &SemanticsChecker,
    map: &mut AliasMap,
) -> Vec<SymbolId> {
    m.params.iter().for_each(|p| map.register(p.1.into()));
    let mut returns = visit_block(&m.body, checker, map);
    m.body
        .find_returns()
        .into_iter()
        .filter_map(|r| r.expression.as_ref())
        .for_each(|expr| returns.extend(visit_expr(&expr, checker, map)));
    map.union_all(&returns);

    if returns.len() > 0 {
        let loc = m.name.0;
        map.signatures
            .insert(loc, AliasSignature::infer_from_method(m, returns, checker));
    }

    vec![]
}
