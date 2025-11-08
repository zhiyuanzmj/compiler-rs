use napi::Either;
use napi::bindgen_prelude::Either3;
use napi::bindgen_prelude::Either4;

use crate::generate::CodegenContext;
use crate::generate::expression::gen_expression;
use crate::generate::utils::CodeFragment;
use crate::generate::utils::FragmentSymbol::Newline;
use crate::generate::utils::gen_call;
use crate::ir::index::DirectiveIRNode;
use crate::ir::index::DirectiveNode;
use crate::ir::index::SimpleExpressionNode;

// This is only for built-in v-model on native elements.
pub fn gen_v_model(oper: DirectiveIRNode, context: &CodegenContext) -> Vec<CodeFragment> {
  let DirectiveIRNode {
    model_type,
    element,
    dir: DirectiveNode { exp, modifiers, .. },
    ..
  } = oper;
  let exp = exp.unwrap();

  let mut result = vec![Either3::A(Newline)];
  let mut body = vec![Either3::C(Some("() => (".to_string()))];

  body.extend(gen_expression(exp.clone(), context, None, None));
  body.push(Either3::C(Some(")".to_string())));

  result.extend(gen_call(
    Either::A(context.helper(match model_type.unwrap().as_str() {
      "text" => "applyTextModel",
      "radio" => "applyRadioModel",
      "checkbox" => "applyCheckboxModel",
      "select" => "applySelectModel",
      "dynamic" => "applyDynamicModel",
      _ => panic!("Unsupported model type"),
    })),
    vec![
      Either4::C(Some(format!("n{element}"))),
      // getter
      Either4::D(body),
      // setter
      Either4::D(gen_model_handler(exp, context)),
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
  result
}

pub fn gen_model_handler(exp: SimpleExpressionNode, context: &CodegenContext) -> Vec<CodeFragment> {
  let mut result = vec![Either3::C(Some("_value => (".to_string()))];
  result.extend(gen_expression(
    exp,
    context,
    Some("_value".to_string()),
    None,
  ));
  result.push(Either3::C(Some(")".to_string())));
  result
}
