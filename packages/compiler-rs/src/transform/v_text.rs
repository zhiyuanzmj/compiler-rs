use std::rc::Rc;

use napi::{
  Result,
  bindgen_prelude::{Either16, JsObjectValue, Object},
};

use crate::{
  ir::index::{BlockIRNode, GetTextChildIRNode, IRNodeTypes, SetTextIRNode},
  transform::{DirectiveTransformResult, TransformContext},
  utils::{
    check::is_void_tag,
    error::{ErrorCodes, on_error},
    expression::{EMPTY_EXPRESSION, get_literal_expression_value, resolve_expression},
    text::get_text,
  },
};

pub fn transform_v_text(
  dir: Object,
  node: Object,
  context: &Rc<TransformContext>,
  context_block: &mut BlockIRNode,
) -> Result<Option<DirectiveTransformResult>> {
  let exp = if let Ok(value) = dir.get_named_property::<Object>("value") {
    resolve_expression(value, context)
  } else {
    on_error(ErrorCodes::X_V_TEXT_NO_EXPRESSION, context);
    EMPTY_EXPRESSION
  };

  if !node
    .get_named_property::<Vec<Object>>("children")?
    .is_empty()
  {
    on_error(ErrorCodes::X_V_TEXT_WITH_CHILDREN, context);
    context.children_template.borrow_mut().clear();
  };

  // v-text on void tags do nothing
  if is_void_tag(&get_text(
    node
      .get_named_property::<Object>("openingElement")?
      .get_named_property("name")?,
    context,
  )) {
    return Ok(None);
  }

  let literal = get_literal_expression_value(&exp);
  if let Some(literal) = literal {
    *context.children_template.borrow_mut() = vec![literal];
  } else {
    *context.children_template.borrow_mut() = vec![" ".to_string()];
    let parent = context.reference(&mut context_block.dynamic)?;
    context.register_operation(
      context_block,
      Either16::P(GetTextChildIRNode {
        get_text_child: true,
        _type: IRNodeTypes::GET_TEXT_CHILD,
        parent,
      }),
      None,
    )?;
    let element = context.reference(&mut context_block.dynamic)?;
    context.register_effect(
      context_block,
      context.is_operation(vec![&exp]),
      Either16::C(SetTextIRNode {
        _type: IRNodeTypes::SET_TEXT,
        set_text: true,
        values: vec![exp],
        element,
        generated: Some(true),
      }),
      None,
      None,
    )?;
  }

  Ok(None)
}
