use napi::Either;
use napi::Result;
use napi::bindgen_prelude::Either3;
use napi::bindgen_prelude::Either4;

use crate::generate::CodegenContext;
use crate::generate::block::gen_block;
use crate::generate::expression::gen_expression;
use crate::generate::utils::CodeFragment;
use crate::generate::utils::FragmentSymbol::Newline;
use crate::generate::utils::gen_call;
use crate::ir::index::BlockIRNode;
use crate::ir::index::IfIRNode;
use crate::utils::my_box::MyBox;

pub fn gen_if(
  oper: IfIRNode,
  context: &CodegenContext,
  context_block: &mut BlockIRNode,
  is_nested: bool,
) -> Result<Vec<CodeFragment>> {
  let IfIRNode {
    condition,
    positive,
    negative,
    once,
    ..
  } = oper;
  let mut frag = vec![];

  let mut condition_expr = vec![Either3::C(Some("() => (".to_string()))];
  condition_expr.extend(gen_expression(condition, context, None, None)?);
  condition_expr.push(Either3::C(Some(")".to_string())));

  let positive_arg = gen_block(positive, context, context_block, vec![], false)?;
  let mut negative_arg: Option<Vec<CodeFragment>> = None;

  if let Some(MyBox(negative)) = negative {
    let negative = *negative;
    negative_arg = Some(match negative {
      Either::A(negative) => gen_block(negative, context, context_block, vec![], false)?,
      Either::B(negative) => {
        let mut result = vec![Either3::C(Some("() => ".to_string()))];
        result.extend(gen_if(negative, context, context_block, true)?);
        result
      }
    });
  }

  if !is_nested {
    frag.push(Either3::A(Newline));
    frag.push(Either3::C(Some(format!("const n{} = ", oper.id))))
  }
  frag.extend(gen_call(
    Either::A(context.helper("createIf")),
    vec![
      Either4::D(condition_expr),
      Either4::D(positive_arg),
      if let Some(negative_arg) = negative_arg {
        Either4::D(negative_arg)
      } else {
        Either4::C(None)
      },
      Either4::C(if once.unwrap_or(false) {
        Some("true".to_string())
      } else {
        None
      }),
    ],
  ));

  Ok(frag)
}
