use napi::Either;
use napi::Env;
use napi::Result;
use napi::bindgen_prelude::Either3;
use napi::bindgen_prelude::Either4;
use napi::bindgen_prelude::Function;
use napi::bindgen_prelude::JsObjectValue;
use napi::bindgen_prelude::Object;
use napi_derive::napi;

use crate::generate::expression::gen_expression;
use crate::generate::utils::CodeFragment;
use crate::generate::utils::FragmentSymbol::Newline;
use crate::generate::utils::gen_call;
use crate::ir::index::DirectiveIRNode;
use crate::ir::index::SimpleExpressionNode;

#[napi]
pub fn gen_v_show(
  env: Env,
  oper: DirectiveIRNode,
  context: Object<'static>,
) -> Result<Vec<CodeFragment>> {
  let DirectiveIRNode { dir, element, .. } = oper;
  let mut result = vec![Either3::A(Newline)];
  let mut body = vec![Either3::C(Some("() => (".to_string()))];
  body.extend(gen_expression(env, dir.exp.unwrap(), context, None, None)?);
  body.push(Either3::C(Some(")".to_string())));
  result.extend(gen_call(
    Either::A(
      context
        .get_named_property::<Function<String, String>>("helper")?
        .call("applyVShow".to_string())?,
    ),
    vec![Either4::C(Some(format!("n{element}"))), Either4::D(body)],
  ));
  Ok(result)
}
