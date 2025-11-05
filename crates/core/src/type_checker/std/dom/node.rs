use crate::types;

use super::utils::*;

pub fn node() -> types::Type {
    let fields: Vec<types::StructField> = vec![
        make_method("baseURI", vec![], types::Type::String),
        make_method("childNodes", vec![], node_array()),
        make_method("firstChild", vec![], node_option()),
        make_method("isConnected", vec![], types::Type::Boolean),
        make_method("lastChild", vec![], node_option()),
        make_method("nextSibling", vec![], node_option()),
        make_method("nodeName", vec![], types::Type::String),
        make_field("nodeValue", string_option()), // getter + setter?
        make_method("ownerDocument", vec![], document_option()),
        make_method("parentNode", vec![], node_option()),
        make_method("parentElement", vec![], element_option()),
        make_method("previousSibling", vec![], node_option()),
        make_field("textContent", string_option()), // getter + setter?
        //
        make_method("appendChild", vec![node_type()], types::Type::Void),
    ];

    types::Type::Duck(types::DuckType {
        like: Box::new(types::StructType { fields }.into()),
    })
}
