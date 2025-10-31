use napi::Either;
use napi::Result;
use napi::bindgen_prelude::Either3;
use napi::bindgen_prelude::Either4;

use crate::generate::CodegenContext;
use crate::generate::utils::CodeFragment;
use crate::generate::utils::FragmentSymbol::Newline;
use crate::generate::utils::gen_call;
use crate::ir::index::InsertNodeIRNode;

pub fn gen_insert_node(
  oper: InsertNodeIRNode,
  context: &CodegenContext,
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
  result.extend(gen_call(
    Either::A(context.helper("insert")),
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
