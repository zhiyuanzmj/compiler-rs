use napi::bindgen_prelude::{Either3, Object};
use napi_derive::napi;

use crate::generate::utils::FragmentSymbol::IndentEnd;
use crate::generate::utils::FragmentSymbol::IndentStart;
use crate::generate::utils::FragmentSymbol::Newline;
use crate::{generate::utils::CodeFragment, ir::index::BlockIRNode};

#[napi]
pub fn gen_block(
  oper: BlockIRNode,
  context: Object<'static>,
  mut args: Vec<CodeFragment>,
  root: bool,
) -> Vec<CodeFragment> {
  let mut result = vec![Either3::C(Some("(".to_string()))];
  result.extend(args);
  result.push(Either3::C(Some(") => {".to_string())));
  result.push(Either3::A(IndentStart));

  // result.append(&mut gen_block_content(oper, context, root));

  result.push(Either3::A(IndentEnd));
  result.push(Either3::A(Newline));
  result.push(Either3::C(Some("}".to_string())));
  result
}
