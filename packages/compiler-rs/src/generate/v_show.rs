use napi::Either;
use napi::Result;
use napi::bindgen_prelude::Either3;
use napi::bindgen_prelude::Either4;

use crate::generate::CodegenContext;
use crate::generate::expression::gen_expression;
use crate::generate::utils::CodeFragment;
use crate::generate::utils::FragmentSymbol::Newline;
use crate::generate::utils::gen_call;
use crate::ir::index::DirectiveIRNode;

pub fn gen_v_show(oper: DirectiveIRNode, context: &CodegenContext) -> Result<Vec<CodeFragment>> {
  let DirectiveIRNode { dir, element, .. } = oper;
  let mut result = vec![Either3::A(Newline)];
  let mut body = vec![Either3::C(Some("() => (".to_string()))];
  body.extend(gen_expression(dir.exp.unwrap(), context, None, None)?);
  body.push(Either3::C(Some(")".to_string())));
  result.extend(gen_call(
    Either::A(context.helper("applyVShow")),
    vec![Either4::C(Some(format!("n{element}"))), Either4::D(body)],
  ));
  Ok(result)
}
