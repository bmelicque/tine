use std::collections::HashMap;
use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;
use tine_ir as ir;
use tine_symbols::{symbols::*, table::SymbolTable};

use crate::codegen::{
    expressions::ExpressionResult,
    utils::{ident_from_str, internal_method_call, member, option},
    CodeGenerator,
};

type MethodTransformer = fn(&mut CodeGenerator, ir::MethodExpression) -> ExpressionResult;

#[derive(Default)]
pub(crate) struct WellknownSymbols {
    pub(crate) methods: HashMap<MethodSymbolId, MethodTransformer>,
}

impl WellknownSymbols {
    pub fn init(&mut self, symbols: &SymbolTable) {
        self.bool(symbols);
        self.float(symbols);
        self.int(symbols);
        self.string(symbols);
        self.array(symbols);
    }

    fn bool(&mut self, symbols: &SymbolTable) {
        let Some(symbol) = symbols.find::<PrimitiveTypeSymbolId, _>(|s| s.name() == "bool") else {
            return;
        };

        self.register_method(symbols, &symbol.methods, "eq", primitive_eq);
        self.register_method(symbols, &symbol.methods, "hash", hash_bool);
    }

    fn float(&mut self, symbols: &SymbolTable) {
        let Some(symbol) = symbols.find::<PrimitiveTypeSymbolId, _>(|s| s.name() == "float") else {
            return;
        };

        self.register_method(symbols, &symbol.methods, "eq", primitive_eq);
        self.register_method(symbols, &symbol.methods, "hash", hash_float);
    }

    fn int(&mut self, symbols: &SymbolTable) {
        let Some(symbol) = symbols.find::<PrimitiveTypeSymbolId, _>(|s| s.name() == "int") else {
            return;
        };

        self.register_method(symbols, &symbol.methods, "eq", primitive_eq);
        self.register_method(symbols, &symbol.methods, "hash", hash_int);
    }

    fn string(&mut self, symbols: &SymbolTable) {
        let Some(symbol) = symbols.find::<PrimitiveTypeSymbolId, _>(|s| s.name() == "string")
        else {
            return;
        };

        self.register_method(symbols, &symbol.methods, "eq", primitive_eq);
        self.register_method(symbols, &symbol.methods, "hash", hash_string);
    }

    fn array(&mut self, symbols: &SymbolTable) {
        let Some(symbol) = symbols
            .find::<StructSymbolId, _>(|s| s.defined_at.module() == 0 && s.name() == "Array")
        else {
            return;
        };

        self.register_method(symbols, &symbol.methods, "length", array_length);
        self.register_method(symbols, &symbol.methods, "get", array_get);
        self.register_method(symbols, &symbol.methods, "set", array_set);
        self.register_method(symbols, &symbol.methods, "pop", array_pop);
    }

    fn register_method(
        &mut self,
        symbols: &SymbolTable,
        methods: &[MethodSymbolId],
        name: &str,
        transformer: MethodTransformer,
    ) {
        let method = methods
            .iter()
            .find(|m| symbols.get(**m).name() == name)
            .cloned();
        if let Some(id) = method {
            self.methods.insert(id, transformer);
        }
    }
}

fn primitive_eq(gen: &mut CodeGenerator, mut method: ir::MethodExpression) -> ExpressionResult {
    let left_result = gen.handle_expression(*method.host);
    let mut prelim_stmts = left_result.prelim_stmts;
    let right_result = gen.handle_expression(method.args.remove(0));
    prelim_stmts.extend(right_result.prelim_stmts);
    let expr = swc::Expr::Bin(swc::BinExpr {
        span: DUMMY_SP,
        op: swc::BinaryOp::EqEqEq,
        left: Box::new(left_result.expr),
        right: Box::new(right_result.expr),
    });

    ExpressionResult { prelim_stmts, expr }
}

fn hash_primitive(
    gen: &mut CodeGenerator,
    method: ir::MethodExpression,
    name: &str,
) -> ExpressionResult {
    let result = gen.handle_expression(*method.host);
    let expr = internal_method_call(name, vec![result.expr.into()]).into();
    ExpressionResult {
        prelim_stmts: result.prelim_stmts,
        expr,
    }
}
fn hash_bool(gen: &mut CodeGenerator, method: ir::MethodExpression) -> ExpressionResult {
    hash_primitive(gen, method, "hashBool")
}
fn hash_float(gen: &mut CodeGenerator, method: ir::MethodExpression) -> ExpressionResult {
    hash_primitive(gen, method, "hashFloat")
}
fn hash_int(gen: &mut CodeGenerator, method: ir::MethodExpression) -> ExpressionResult {
    hash_primitive(gen, method, "hashInt")
}
fn hash_string(gen: &mut CodeGenerator, method: ir::MethodExpression) -> ExpressionResult {
    hash_primitive(gen, method, "hashString")
}

fn array_length(gen: &mut CodeGenerator, method: ir::MethodExpression) -> ExpressionResult {
    let array = gen.handle_expression(*method.host);
    let prelim_stmts = array.prelim_stmts;
    let expr = swc::Expr::Member(swc::MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(array.expr),
        prop: swc::MemberProp::Ident(ident_from_str("length").into()),
    });
    ExpressionResult { prelim_stmts, expr }
}

fn array_get(gen: &mut CodeGenerator, mut method: ir::MethodExpression) -> ExpressionResult {
    let array = gen.handle_expression(*method.host);
    let mut prelim_stmts = array.prelim_stmts;
    let index = gen.handle_expression(method.args.remove(0));
    prelim_stmts.extend(index.prelim_stmts);
    let expr = swc::Expr::Call(swc::CallExpr {
        callee: swc::Callee::Expr(Box::new(member(array.expr, "at").into())),
        args: vec![index.expr.into()],
        ..Default::default()
    });
    let expr = option(expr);
    ExpressionResult { prelim_stmts, expr }
}

fn array_set(gen: &mut CodeGenerator, mut method: ir::MethodExpression) -> ExpressionResult {
    let array = gen.handle_expression(*method.host);
    let mut prelim_stmts = array.prelim_stmts;
    let index = gen.handle_expression(method.args.remove(0));
    prelim_stmts.extend(index.prelim_stmts);
    let value = gen.handle_expression(method.args.remove(0));
    prelim_stmts.extend(value.prelim_stmts);
    let expr = swc::Expr::Call(internal_method_call(
        "setArrayAt",
        vec![array.expr.into(), index.expr.into(), value.expr.into()],
    ));
    ExpressionResult { prelim_stmts, expr }
}

fn array_pop(gen: &mut CodeGenerator, method: ir::MethodExpression) -> ExpressionResult {
    let array = gen.handle_expression(*method.host);
    let prelim_stmts = array.prelim_stmts;
    let expr = swc::Expr::Call(swc::CallExpr {
        callee: swc::Callee::Expr(Box::new(member(array.expr, "pop").into())),
        ..Default::default()
    });
    let expr = option(expr);
    ExpressionResult { prelim_stmts, expr }
}
