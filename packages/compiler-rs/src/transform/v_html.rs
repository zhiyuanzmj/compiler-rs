use std::rc::Rc;

use napi::{
  Result,
  bindgen_prelude::{Either16, JsObjectValue, Object},
};

use crate::{
  ir::index::{BlockIRNode, IRNodeTypes, SetHtmlIRNode},
  transform::{DirectiveTransformResult, TransformContext},
  utils::{
    error::{ErrorCodes, on_error},
    expression::{EMPTY_EXPRESSION, get_value, resolve_expression},
  },
};

pub fn transform_v_html(
  dir: Object,
  node: Object,
  context: &Rc<TransformContext>,
  context_block: &mut BlockIRNode,
) -> Result<Option<DirectiveTransformResult>> {
  let exp = if let Some(value) = get_value::<Object>(dir) {
    resolve_expression(value, context)
  } else {
    on_error(ErrorCodes::X_V_HTML_NO_EXPRESSION, context);
    EMPTY_EXPRESSION
  };

  if let Some(children) = node.get_named_property::<Vec<Object>>("children").ok() {
    if children.len() != 0 {
      on_error(ErrorCodes::X_V_HTML_WITH_CHILDREN, context);
    }
    context.children_template.borrow_mut().clear();
  }

  let element = context.reference(&mut context_block.dynamic)?;
  context.register_effect(
    context_block,
    context.is_operation(vec![&exp]),
    Either16::I(SetHtmlIRNode {
      set_html: true,
      _type: IRNodeTypes::SET_HTML,
      element,
      value: exp,
    }),
    None,
    None,
  )?;

  Ok(None)
}
