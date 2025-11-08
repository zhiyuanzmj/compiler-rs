use napi::Either;
use napi::bindgen_prelude::Either3;
use napi::bindgen_prelude::Either4;

use crate::generate::CodegenContext;
use crate::generate::expression::gen_expression;
use crate::generate::utils::CodeFragment;
use crate::generate::utils::FragmentSymbol::Newline;
use crate::generate::utils::gen_call;
use crate::ir::index::CreateNodesIRNode;
use crate::ir::index::GetTextChildIRNode;
use crate::ir::index::SetNodesIRNode;
use crate::ir::index::SetTextIRNode;
use crate::ir::index::SimpleExpressionNode;
use crate::utils::check::is_constant_node;

pub fn gen_set_text(oper: SetTextIRNode, context: &CodegenContext) -> Vec<CodeFragment> {
  let SetTextIRNode {
    element,
    values,
    generated,
    ..
  } = oper;
  let texts = combine_values(values, context, true, true);
  let mut result = vec![Either3::A(Newline)];
  result.extend(gen_call(
    Either::A(context.helper("setText")),
    vec![
      Either4::C(Some(format!(
        "{}{}",
        if generated.unwrap_or(false) { "x" } else { "n" },
        element
      ))),
      Either4::D(texts),
    ],
  ));
  result
}

pub fn gen_get_text_child(oper: GetTextChildIRNode, context: &CodegenContext) -> Vec<CodeFragment> {
  vec![
    Either3::A(Newline),
    Either3::C(Some(format!(
      "const x{} = {}(n{})",
      oper.parent,
      context.helper("child"),
      oper.parent
    ))),
  ]
}

pub fn gen_set_nodes(oper: SetNodesIRNode, context: &CodegenContext) -> Vec<CodeFragment> {
  let SetNodesIRNode {
    element,
    values,
    generated,
    once,
    ..
  } = oper;
  let mut result = vec![Either3::A(Newline)];
  result.extend(gen_call(
    Either::A(context.helper("setNodes")),
    vec![
      Either4::C(Some(format!(
        "{}{}",
        if generated.unwrap_or(false) {
          "x".to_string()
        } else {
          "n".to_string()
        },
        element
      ))),
      Either4::D(combine_values(values, context, once, false)),
    ],
  ));
  result
}

pub fn gen_create_nodes(oper: CreateNodesIRNode, context: &CodegenContext) -> Vec<CodeFragment> {
  let CreateNodesIRNode {
    id, values, once, ..
  } = oper;
  let mut result = vec![
    Either3::A(Newline),
    Either3::C(Some(format!("const n{id} = "))),
  ];
  result.extend(gen_call(
    Either::A(context.helper("createNodes")),
    vec![Either4::D(combine_values(values, context, once, false))],
  ));
  result
}

fn combine_values(
  values: Vec<SimpleExpressionNode>,
  context: &CodegenContext,
  once: bool,
  is_set_text: bool,
) -> Vec<CodeFragment> {
  let mut i = 0;
  values
    .into_iter()
    .flat_map(move |value| {
      let should_wrap = !once
        && !is_set_text
        && !value.content.is_empty()
        && !value.is_static
        && !is_constant_node(&value.ast.as_ref());
      let literal_expression_value = &value.get_literal_expression_value();
      let mut exp = gen_expression(value, context, None, Some(should_wrap));
      if is_set_text && literal_expression_value.is_none() {
        // dynamic, wrap with toDisplayString
        exp = gen_call(
          Either::A(context.helper("toDisplayString")),
          vec![Either4::D(exp)],
        )
      }
      if i > 0 {
        exp.insert(
          0,
          Either3::C(Some(if is_set_text {
            " + ".to_string()
          } else {
            ", ".to_string()
          })),
        )
      }
      i += 1;
      exp
    })
    .collect()
}
