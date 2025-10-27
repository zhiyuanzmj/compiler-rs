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
use crate::ir::index::DirectiveNode;
use crate::ir::index::SimpleExpressionNode;

#[napi]
// This is only for built-in v-model on native elements.
pub fn gen_v_model(
  env: Env,
  oper: DirectiveIRNode,
  context: Object<'static>,
) -> Result<Vec<CodeFragment>> {
  let DirectiveIRNode {
    model_type,
    element,
    dir: DirectiveNode { exp, modifiers, .. },
    ..
  } = oper;
  let exp = exp.unwrap();

  let mut result = vec![Either3::A(Newline)];
  let mut body = vec![Either3::C(Some("() => (".to_string()))];

  body.extend(gen_expression(env, exp.clone(), context, None, None)?);
  body.push(Either3::C(Some(")".to_string())));

  result.extend(gen_call(
    Either::A(
      context
        .get_named_property::<Function<String, String>>("helper")?
        .call(match model_type.unwrap().as_str() {
          "text" => "applyTextModel".to_string(),
          "radio" => "applyRadioModel".to_string(),
          "checkbox" => "applyCheckboxModel".to_string(),
          "select" => "applySelectModel".to_string(),
          "dynamic" => "applyDynamicModel".to_string(),
          _ => panic!("Unsupported model type"),
        })?,
    ),
    vec![
      Either4::C(Some(format!("n{element}"))),
      // getter
      Either4::D(body),
      // setter
      Either4::D(gen_model_handler(env, exp, context)?),
      // modifiers
      if modifiers.len() > 0 {
        Either4::C(Some(format!(
          "{{ {} }}",
          modifiers
            .into_iter()
            .map(|e| format!("{}: true", e.content))
            .collect::<Vec<String>>()
            .join(", ")
        )))
      } else {
        Either4::C(None)
      },
    ],
  ));
  Ok(result)
}

#[napi]
pub fn gen_model_handler(
  env: Env,
  exp: SimpleExpressionNode,
  context: Object,
) -> Result<Vec<CodeFragment>> {
  let mut result = vec![Either3::C(Some("_value => (".to_string()))];
  result.extend(gen_expression(
    env,
    exp,
    context,
    Some("_value".to_string()),
    None,
  )?);
  result.push(Either3::C(Some(")".to_string())));
  Ok(result)
}
