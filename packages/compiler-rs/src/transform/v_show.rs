use std::rc::Rc;

use napi::{
  Result,
  bindgen_prelude::{Either18, Object},
};

use crate::{
  ir::index::{BlockIRNode, DirectiveIRNode, IRNodeTypes},
  transform::{DirectiveTransformResult, TransformContext},
  utils::{
    directive::resolve_directive,
    error::{ErrorCodes, on_error},
    expression::EMPTY_EXPRESSION,
  },
};

pub fn transform_v_show(
  _dir: Object,
  _: Object,
  context: &Rc<TransformContext>,
  context_block: &mut BlockIRNode,
) -> Result<Option<DirectiveTransformResult>> {
  let mut dir = resolve_directive(_dir, context)?;
  if dir.exp.is_none() {
    on_error(ErrorCodes::X_V_SHOW_NO_EXPRESSION, context);
    dir.exp = Some(EMPTY_EXPRESSION)
  }

  let element = context.reference(&mut context_block.dynamic)?;
  context.register_operation(
    context_block,
    Either18::N(DirectiveIRNode {
      _type: IRNodeTypes::DIRECTIVE,
      element,
      dir,
      name: String::from("show"),
      builtin: Some(true),
      asset: None,
      model_type: None,
    }),
    None,
  )?;
  Ok(None)
}
