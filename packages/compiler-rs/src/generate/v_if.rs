use napi::Either;
use napi::Env;
use napi::Result;
use napi::bindgen_prelude::Either3;
use napi::bindgen_prelude::Either4;
use napi::bindgen_prelude::Function;
use napi::bindgen_prelude::JsObjectValue;
use napi::bindgen_prelude::Object;
use napi_derive::napi;

use crate::generate::block::gen_block;
use crate::generate::expression::gen_expression;
use crate::generate::utils::CodeFragment;
use crate::generate::utils::FragmentSymbol::Newline;
use crate::generate::utils::gen_call;
use crate::ir::index::DirectiveIRNode;
use crate::ir::index::IfIRNode;

#[napi]
pub fn gen_if(
  env: Env,
  oper: IfIRNode,
  context: Object<'static>,
  is_nested: bool,
) -> Result<Vec<CodeFragment>> {
  let IfIRNode {
    condition,
    positive,
    negative,
    once,
    ..
  } = oper;
  let frag = vec![];

  let mut condition_exp = vec![Either3::C(Some("() => (".to_string()))];
  condition_exp.extend(gen_expression(env, condition, context, None, None)?);
  condition_exp.push(Either3::C(Some(")".to_string())));

  let positive_arg = gen_block(positive, context, vec![], false);
  // let negative_arg = None;

  Ok(frag)
}
