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
use crate::ir::index::SetHtmlIRNode;

#[napi]
pub fn gen_set_html(
  env: Env,
  oper: SetHtmlIRNode,
  context: Object<'static>,
) -> Result<Vec<CodeFragment>> {
  let SetHtmlIRNode { value, element, .. } = oper;

  let mut result = vec![Either3::A(Newline)];
  result.extend(gen_call(
    Either::A(
      context
        .get_named_property::<Function<String, String>>("helper")?
        .call("setHtml".to_string())?,
    ),
    vec![
      Either4::C(Some(format!("n{element}"))),
      Either4::D(gen_expression(env, value, context, None, None)?),
    ],
  ));
  Ok(result)
}
