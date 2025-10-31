use napi::Either;
use napi::Result;
use napi::bindgen_prelude::Either3;
use napi::bindgen_prelude::Either4;

use crate::generate::CodegenContext;
use crate::generate::expression::gen_expression;
use crate::generate::utils::CodeFragment;
use crate::generate::utils::FragmentSymbol::Newline;
use crate::generate::utils::gen_call;
use crate::ir::index::SetHtmlIRNode;

pub fn gen_set_html(oper: SetHtmlIRNode, context: &CodegenContext) -> Result<Vec<CodeFragment>> {
  let SetHtmlIRNode { value, element, .. } = oper;

  let mut result = vec![Either3::A(Newline)];
  result.extend(gen_call(
    Either::A(context.helper("setHtml")),
    vec![
      Either4::C(Some(format!("n{element}"))),
      Either4::D(gen_expression(value, context, None, None)?),
    ],
  ));
  Ok(result)
}
