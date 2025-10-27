use std::rc::Rc;

use napi::{
  Either, Result,
  bindgen_prelude::{Either16, JsObjectValue, Object},
};

use crate::{
  ir::index::{BlockIRNode, DynamicFlag, IRDynamicInfo, IRNodeTypes, IfIRNode},
  transform::TransformContext,
  utils::{
    check::{_is_constant_node, is_template},
    directive::resolve_directive,
    error::{ErrorCodes, on_error},
    expression::create_simple_expression,
    my_box::MyBox,
    utils::find_prop,
  },
};

pub fn transform_v_if<'a>(
  node: Object<'static>,
  context: &'a Rc<TransformContext>,
  context_block: &'a mut BlockIRNode,
  parent_dynamic: &'a mut IRDynamicInfo,
) -> Result<Option<Box<dyn FnOnce() -> Result<()> + 'a>>> {
  if !node.get_named_property::<String>("type")?.eq("JSXElement")
    || (is_template(&node) && find_prop(&node, Either::A("v-slot".to_string())).is_some())
  {
    return Ok(None);
  }
  let Some(prop) = find_prop(
    &node,
    Either::B(vec![
      "v-if".to_string(),
      "v-else".to_string(),
      "v-else-if".to_string(),
    ]),
  ) else {
    return Ok(None);
  };
  let seen = &mut context.seen.borrow_mut();
  let start = prop.get_named_property::<i32>("start")?;
  if seen.contains(&start) {
    return Ok(None);
  }
  seen.insert(start);

  let mut dir = resolve_directive(prop, context)?;
  if dir.name != "else"
    && (dir.exp.is_none() || dir.exp.as_ref().unwrap().content.trim().is_empty())
  {
    on_error(ErrorCodes::X_V_IF_NO_EXPRESSION, context);
    dir.exp = Some(create_simple_expression(
      "true".to_string(),
      Some(false),
      None,
      None,
    ));
  }

  let dynamic = &mut context_block.dynamic;
  dynamic.flags |= DynamicFlag::NON_TEMPLATE as i32;

  if dir.name == "if" {
    let id = context.reference(dynamic)?;
    dynamic.flags |= DynamicFlag::INSERT as i32;
    let block = context_block as *mut BlockIRNode;
    let exit_block = context.create_block(unsafe { &mut *block }, node, None)?;
    return Ok(Some(Box::new(move || {
      let block = exit_block()?;

      context_block.dynamic.operation = Some(MyBox(Box::new(Either16::A(IfIRNode {
        _type: IRNodeTypes::IF,
        id,
        positive: block,
        once: Some(
          *context.in_v_once.borrow() || _is_constant_node(&dir.exp.as_ref().unwrap().ast),
        ),
        condition: dir.exp.unwrap(),
        negative: None,
        anchor: None,
        parent: None,
      }))));

      Ok(())
    })));
  }

  let siblings = &mut parent_dynamic.children;
  // let mut siblings: Vec<IRDynamicInfo> = vec![];
  let mut last_if_node = None;
  if siblings.len() > 0 {
    let mut i = siblings.len();
    while i > 0 {
      i = i - 1;
      let sibling = siblings.get_mut(i).unwrap() as *mut IRDynamicInfo;
      if let Some(MyBox(operation)) = (unsafe { &mut *sibling }).operation.as_mut()
        && let Either16::A(operation) = operation.as_mut()
      {
        last_if_node = Some(operation);
        break;
      }
    }
  }

  // check if IfNode is the last operation and get the root IfNode
  let Some(mut last_if_node) = last_if_node else {
    on_error(ErrorCodes::X_V_ELSE_NO_ADJACENT_IF, context);
    return Ok(None);
  };

  let mut last_if_node_ptr = last_if_node as *mut IfIRNode;
  while let Some(MyBox(negative)) = (unsafe { &mut *last_if_node_ptr }).negative.as_mut()
    && let Either::B(negative) = negative.as_mut()
  {
    last_if_node_ptr = negative as *mut IfIRNode;
  }
  last_if_node = unsafe { &mut *last_if_node_ptr };

  // Check if v-else was followed by v-else-if
  if dir.name == "else-if" && last_if_node.negative.is_some() {
    on_error(ErrorCodes::X_V_ELSE_NO_ADJACENT_IF, context);
  };

  let exit_block = context.create_block(context_block, node, None)?;

  Ok(Some(Box::new(move || {
    let block = exit_block()?;
    if dir.name == "else" {
      last_if_node.negative = Some(MyBox(Box::new(Either::A(block))));
    } else {
      last_if_node.negative = Some(MyBox(Box::new(Either::B(IfIRNode {
        _type: IRNodeTypes::IF,
        id: -1,
        positive: block,
        once: Some(
          *context.in_v_once.borrow() || _is_constant_node(&dir.exp.as_ref().unwrap().ast),
        ),
        condition: dir.exp.unwrap(),
        parent: None,
        anchor: None,
        negative: None,
      }))))
    }
    Ok(())
  })))
}
