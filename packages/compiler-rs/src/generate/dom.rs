use napi::Either;
use napi::Result;
use napi::bindgen_prelude::Either3;
use napi::bindgen_prelude::Either4;
use napi::bindgen_prelude::Function;
use napi::bindgen_prelude::JsObjectValue;
use napi::bindgen_prelude::Object;
use napi_derive::napi;

use crate::generate::utils::CodeFragment;
use crate::generate::utils::FragmentSymbol::Newline;
use crate::generate::utils::gen_call;
use crate::ir::index::InsertNodeIRNode;

#[napi]
pub fn gen_insert_node(
  oper: InsertNodeIRNode,
  context: Object<'static>,
) -> Result<Vec<CodeFragment>> {
  let InsertNodeIRNode {
    parent,
    elements,
    anchor,
    ..
  } = oper;
  let mut element = elements
    .iter()
    .map(|el| format!("n{el}"))
    .collect::<Vec<String>>()
    .join(", ");
  if elements.len() > 1 {
    element = format!("[{element}]");
  }
  let mut result = vec![Either3::A(Newline)];
  result.append(&mut gen_call(
    Either::A(
      context
        .get_named_property::<Function<String, String>>("helper")?
        .call("insert".to_string())?,
    ),
    vec![
      Either4::C(Some(element)),
      Either4::C(Some(format!("n{parent}"))),
      if let Some(anchor) = anchor {
        Either4::C(Some(format!("n{}", anchor.to_string())))
      } else {
        Either4::C(None)
      },
    ],
  ));
  Ok(result)
}
