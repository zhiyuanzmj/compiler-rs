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
use crate::ir::index::DeclareOldRefIRNode;
use crate::ir::index::SetTemplateRefIRNode;

#[napi]
pub fn gen_set_template_ref(
  env: Env,
  oper: SetTemplateRefIRNode,
  context: Object,
) -> Result<Vec<CodeFragment>> {
  let SetTemplateRefIRNode {
    effect,
    element,
    value,
    ref_for,
    ..
  } = oper;

  let mut result = vec![
    Either3::A(Newline),
    Either3::C(if effect {
      Some(format!("r{element} = "))
    } else {
      None
    }),
  ];
  result.extend(gen_call(
    Either::A("_setTemplateRef".to_string()), // will be generated in root scope
    vec![
      Either4::C(Some(format!("n{element}"))),
      Either4::D(gen_expression(env, value, context, None, None)?),
      Either4::C(if effect {
        Some(format!("r{element}"))
      } else if ref_for {
        Some("void 0".to_string())
      } else {
        None
      }),
      Either4::C(if ref_for {
        Some("true".to_string())
      } else {
        None
      }),
    ],
  ));
  Ok(result)
}

#[napi]
pub fn gen_declare_old_ref(oper: DeclareOldRefIRNode) -> Vec<CodeFragment> {
  vec![
    Either3::A(Newline),
    Either3::C(Some(format!("let r{}", oper.id))),
  ]
}
