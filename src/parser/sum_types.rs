// use pest::iterators::Pair;

// use crate::ast::{Node, Spanned, SumTypeConstructor};

// use super::{
//     parser::{ParseError, ParseResult, Rule},
//     types::parse_type,
// };

// pub fn parse_sum_type(pair: Pair<'static, Rule>) -> ParseResult {
//     let span = pair.clone().as_span();
//     let mut inner = pair.into_inner();

//     let mut constructors = Vec::new();
//     let mut errors = Vec::new();
//     while let Some(pair) = inner.next() {
//         assert!(pair.as_rule() == Rule::sum_constructor);
//         let mut result = parse_sum_constructor(pair);
//         errors.append(&mut result.1);
//         constructors.push(result.0);
//     }

//     // FIXME:
//     ParseResult {
//         node: Some(Spanned {
//             node: Node::SumType(constructors),
//             span,
//         }),
//         errors,
//     }
// }

// fn parse_sum_constructor(pair: Pair<'static, Rule>) -> (SumTypeConstructor, Vec<ParseError>) {
//     let mut inner = pair.into_inner();

//     let mut name = None;
//     let mut param = None;
//     let mut errors = Vec::<ParseError>::new();
//     while let Some(pair) = inner.next() {
//         match pair.as_rule() {
//             Rule::identifier => name = Some(pair.as_str().to_string()),
//             Rule::sum_param => {
//                 if let Some(inner) = pair.into_inner().next() {
//                     let mut result = parse_type(inner);
//                     param = result.node;
//                     errors.append(&mut result.errors);
//                 };
//             }
//             Rule::struct_body => {
//                 // TODO: parse_struct_body
//             }
//             rule => unreachable!("Unexpected rule in sum_constructor (found '{:?}')", rule),
//         }
//     }

//     (
//         SumTypeConstructor {
//             name: name.unwrap(),
//             param,
//         },
//         errors,
//     )
// }
