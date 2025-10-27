use std::rc::Rc;

use napi::{
  Either, Result,
  bindgen_prelude::{Either16, JsObjectValue, Object},
};

use crate::{
  ir::index::{
    BlockIRNode, DynamicFlag, ForIRNode, IRDynamicInfo, IRFor, IRNodeTypes, SimpleExpressionNode,
  },
  transform::TransformContext,
  utils::{
    check::{_is_constant_node, is_jsx_component, is_template},
    error::{ErrorCodes, on_error},
    expression::resolve_expression,
    my_box::MyBox,
    text::is_empty_text,
    utils::{find_prop, get_expression},
  },
};

pub fn transform_v_for<'a>(
  node: Object<'static>,
  context: &'a Rc<TransformContext>,
  context_block: &'a mut BlockIRNode,
  _: &'a mut IRDynamicInfo,
) -> Result<Option<Box<dyn FnOnce() -> Result<()> + 'a>>> {
  if !node.get_named_property::<String>("type")?.eq("JSXElement")
    || (is_template(&node) && find_prop(&node, Either::A("v-slot".to_string())).is_some())
  {
    return Ok(None);
  }
  let Some(dir) = find_prop(&node, Either::A("v-for".to_string())) else {
    return Ok(None);
  };
  let seen = &mut context.seen.borrow_mut();
  let start = dir.get_named_property::<i32>("start")?;
  if seen.contains(&start) {
    return Ok(None);
  }
  seen.insert(start);

  let component = is_jsx_component(node) || is_template_with_single_component(node)?;
  let dynamic = &mut context_block.dynamic;
  let id = context.reference(dynamic)?;
  dynamic.flags = dynamic.flags | DynamicFlag::NON_TEMPLATE as i32 | DynamicFlag::INSERT as i32;
  let block = context_block as *mut BlockIRNode;
  let exit_block = context.create_block(unsafe { &mut *block }, node, Some(true))?;
  Ok(Some(Box::new(move || {
    let block = exit_block()?;

    let parent = context.parent.borrow().upgrade().unwrap();
    let Some(dir) = find_prop(&node, Either::A("v-for".to_string())) else {
      return Ok(());
    };
    let IRFor {
      value,
      index,
      key,
      source,
    } = get_for_parse_result(dir, context)?;
    let Some(source) = source else {
      on_error(ErrorCodes::X_V_FOR_MALFORMED_EXPRESSION, context);
      return Ok(());
    };

    let key_prop = find_prop(&node, Either::A("key".to_string()));
    let key_prop = if let Some(key_prop) = key_prop
      && key_prop
        .get_named_property::<String>("type")?
        .eq("JSXAttribute")
      && let Ok(value) = key_prop.get_named_property::<Object>("value")
    {
      Some(resolve_expression(value, context))
    } else {
      None
    };

    // if v-for is the only child of a parent element, it can go the fast path
    // when the entire list is emptied
    let only_child = context_block.node.is_some()
      && !context
        .env
        .strict_equals(context_block.node.unwrap(), *parent.node.borrow())?
      && parent
        .node
        .borrow()
        .get_named_property::<Vec<Object>>("children")?
        .into_iter()
        .filter(|child| !is_empty_text(*child))
        .collect::<Vec<Object>>()
        .len()
        == 1;

    context_block.dynamic.operation = Some(MyBox(Box::new(Either16::B(ForIRNode {
      _type: IRNodeTypes::FOR,
      id,
      value,
      key,
      index,
      key_prop,
      render: block,
      once: *context.in_v_once.borrow() || _is_constant_node(&source.ast),
      source,
      component,
      only_child,
      parent: None,
      anchor: None,
    }))));

    Ok(())
  })))
}

pub fn get_for_parse_result(dir: Object, context: &Rc<TransformContext>) -> Result<IRFor> {
  let mut value: Option<SimpleExpressionNode> = None;
  let mut index: Option<SimpleExpressionNode> = None;
  let mut key: Option<SimpleExpressionNode> = None;
  let mut source: Option<SimpleExpressionNode> = None;
  if let Ok(dir_value) = dir.get_named_property::<Object>("value") {
    let expression = if dir_value
      .get_named_property::<String>("type")?
      .eq("JSXExpressionContainer")
    {
      Some(get_expression(dir_value))
    } else {
      None
    };
    if let Some(expression) = expression
      && expression
        .get_named_property::<String>("type")
        .unwrap()
        .eq("BinaryExpression")
    {
      let left = get_expression(expression.get_named_property::<Object>("left")?);
      if left
        .get_named_property::<String>("type")?
        .eq("SequenceExpression")
      {
        let mut expressions = left.get_named_property::<Vec<Object>>("expressions")?;
        value = expressions
          .get_mut(0)
          .map(|e| resolve_expression(*e, context));
        key = expressions
          .get_mut(1)
          .map(|e| resolve_expression(*e, context));
        index = expressions
          .get_mut(2)
          .map(|e| resolve_expression(*e, context))
      } else {
        value = Some(resolve_expression(left, context));
      };
      source = Some(resolve_expression(
        expression.get_named_property::<Object>("right")?,
        context,
      ));
    }
  } else {
    on_error(ErrorCodes::X_V_FOR_NO_EXPRESSION, context);
  }
  return Ok(IRFor {
    value,
    index,
    key,
    source,
  });
}

fn is_template_with_single_component(node: Object) -> Result<bool> {
  let non_comment_children: Vec<Object> = node
    .get_named_property::<Vec<Object>>("children")?
    .into_iter()
    .filter(|c| !is_empty_text(*c))
    .collect();
  Ok(non_comment_children.len() == 1 && is_jsx_component(*non_comment_children.get(0).unwrap()))
}
