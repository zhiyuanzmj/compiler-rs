use std::rc::Rc;

use napi::{
  Either,
  bindgen_prelude::{Either18, JsObjectValue, Object, Result},
};

use crate::{
  ir::index::{
    BlockIRNode, CreateNodesIRNode, DynamicFlag, GetTextChildIRNode, IRDynamicInfo, IRNodeTypes,
    IfIRNode, SetNodesIRNode, SimpleExpressionNode,
  },
  transform::TransformContext,
  utils::{
    check::{_is_constant_node, is_fragment_node, is_jsx_component, is_template},
    expression::{_get_literal_expression_value, resolve_expression},
    my_box::MyBox,
    text::{is_empty_text, resolve_jsx_text},
    utils::{_get_expression, find_prop, get_expression},
  },
};

pub fn transform_text<'a>(
  node: Object<'static>,
  context: &'a Rc<TransformContext>,
  context_block: &'a mut BlockIRNode,
  parent_dynamic: &'a mut IRDynamicInfo,
) -> Result<Option<Box<dyn FnOnce() -> Result<()> + 'a>>> {
  let dynamic = &mut context_block.dynamic;
  if let Ok(start) = node.get_named_property::<i32>("start") {
    let seen = &mut context.seen.borrow_mut();
    if seen.contains(&start) {
      dynamic.flags |= DynamicFlag::NON_TEMPLATE as i32;
      return Ok(None);
    }
  }

  let children = node.get_named_property::<Vec<Object<'static>>>("children");
  let is_fragment = is_fragment_node(&node);
  let node_type = node.get_named_property::<String>("type")?;
  if ((node_type.eq("JSXElement") && !is_template(&node) && !is_jsx_component(node)) || is_fragment)
    && let Ok(children) = children
    && children.len() > 0
  {
    let mut has_interp = false;
    let mut is_all_text_like = true;
    for child in &children {
      let child_type = child.get_named_property::<String>("type")?;
      if child_type.eq("JSXExpressionContainer") {
        let exp_type = _get_expression(child).get_named_property::<String>("type")?;
        if exp_type != "ConditionalExpression" && exp_type != "LogicalExpression" {
          has_interp = true
        }
      } else if child_type != "JSXText" {
        is_all_text_like = false
      }
    }

    // all text like with interpolation
    if !is_fragment && is_all_text_like && has_interp {
      process_text_container(children, context, context_block)?
    } else if has_interp {
      // check if there's any text before interpolation, it needs to be merged
      let mut i = 0;
      for child in &children {
        let prev = if i > 0 { children.get(i - 1) } else { None };
        if child
          .get_named_property::<String>("type")?
          .eq("JSXExpressionContainer")
          && let Some(prev) = prev
          && prev.get_named_property::<String>("type")?.eq("JSXText")
        {
          // mark leading text node for skipping
          mark_non_template(*prev, context)?;
        }
        i = i + 1;
      }
    }
  } else if node_type.eq("JSXExpressionContainer") {
    let expression = get_expression(node);
    let expression_type = expression.get_named_property::<String>("type")?;
    if expression_type.eq("ConditionalExpression") {
      return Ok(Some(process_conditional_expression(
        expression,
        context,
        context_block,
        parent_dynamic,
      )?));
    } else if expression_type.eq("LogicalExpression") {
      return Ok(Some(process_logical_expression(
        expression,
        context,
        context_block,
        parent_dynamic,
      )?));
    } else {
      process_interpolation(context, context_block)?;
    }
  } else if node_type == "JSXText" {
    let value = resolve_jsx_text(node);
    if !value.is_empty() {
      let mut template = context.template.borrow_mut();
      *template = template.to_string() + &value;
    } else {
      dynamic.flags |= DynamicFlag::NON_TEMPLATE as i32;
    }
  }
  Ok(None)
}

fn process_interpolation(
  context: &Rc<TransformContext>,
  context_block: &mut BlockIRNode,
) -> Result<()> {
  let children = context
    .parent
    .borrow()
    .upgrade()
    .unwrap()
    .node
    .borrow()
    .get_named_property::<Vec<Object>>("children")?;
  let index = context.index as usize;
  let nexts = children[index..].to_vec();
  let idx = nexts.iter().position(|n| !is_text_like(n).unwrap_or(false));
  let mut nodes = if let Some(idx) = idx {
    nexts[..idx].to_vec()
  } else {
    nexts
  };

  // merge leading text
  let prev = if index > 0 {
    children.get(index - 1)
  } else {
    None
  };
  if let Some(prev) = prev
    && prev.get_named_property::<String>("type")?.eq("JSXText")
  {
    nodes.insert(0, *prev);
  }

  let values = process_text_like_expressions(nodes, context)?;
  let dynamic = &mut context_block.dynamic;
  if values.is_empty() {
    dynamic.flags |= DynamicFlag::NON_TEMPLATE as i32;
    return Ok(());
  }

  let id = context.reference(dynamic)?;
  let once = *context.in_v_once.borrow();
  if is_fragment_node(&context.parent.borrow().upgrade().unwrap().node.borrow())
    || find_prop(
      &context.parent.borrow().upgrade().unwrap().node.borrow(),
      Either::A(String::from("v-slot")),
    )
    .is_some()
  {
    context.register_operation(
      context_block,
      Either18::K(CreateNodesIRNode {
        _type: IRNodeTypes::CREATE_NODES,
        id,
        once,
        values,
      }),
      None,
    )?;
  } else {
    let mut template = context.template.borrow_mut();
    *template = template.to_string() + " ";
    context.register_operation(
      context_block,
      Either18::G(SetNodesIRNode {
        _type: IRNodeTypes::SET_NODES,
        element: id,
        once,
        values,
        generated: None,
      }),
      None,
    )?;
  }
  Ok(())
}

fn mark_non_template(node: Object, context: &Rc<TransformContext>) -> Result<()> {
  let seen = &mut context.seen.borrow_mut();
  seen.insert(node.get_named_property::<i32>("start")?);
  Ok(())
}

fn process_text_container(
  children: Vec<Object<'static>>,
  context: &Rc<TransformContext>,
  context_block: &mut BlockIRNode,
) -> Result<()> {
  let values = process_text_like_expressions(children, context)?;
  let literals = values
    .iter()
    .map(_get_literal_expression_value)
    .collect::<Vec<Option<String>>>();
  if literals.iter().all(|l| l.is_some()) {
    *context.children_template.borrow_mut() = literals.into_iter().filter_map(|i| i).collect();
  } else {
    *context.children_template.borrow_mut() = vec![" ".to_string()];
    let parent = context.reference(&mut context_block.dynamic)?;
    context.register_operation(
      context_block,
      Either18::R(GetTextChildIRNode {
        _type: IRNodeTypes::GET_TEXT_CHILD,
        parent,
      }),
      None,
    )?;
    let element = context.reference(&mut context_block.dynamic)?;
    context.register_operation(
      context_block,
      Either18::G(SetNodesIRNode {
        _type: IRNodeTypes::SET_NODES,
        element,
        once: *context.in_v_once.borrow(),
        values,
        // indicates this node is generated, so prefix should be "x" instead of "n"
        generated: Some(true),
      }),
      None,
    )?;
  }
  Ok(())
}

fn process_text_like_expressions(
  nodes: Vec<Object<'static>>,
  context: &Rc<TransformContext>,
) -> Result<Vec<SimpleExpressionNode>> {
  let mut values = vec![];
  for node in nodes {
    mark_non_template(node, context)?;
    if is_empty_text(node) {
      continue;
    }
    values.push(resolve_expression(node, context))
  }
  Ok(values)
}

fn is_text_like(node: &Object) -> Result<bool> {
  let node_type = node.get_named_property::<String>("type")?;
  Ok(if node_type == "JSXExpressionContainer" {
    let expression_type = _get_expression(node).get_named_property::<String>("type")?;
    expression_type != "ConditionalExpression" && expression_type != "LogicalExpression"
  } else {
    node_type == "JSXText"
  })
}

pub fn process_conditional_expression<'a>(
  node: Object<'static>,
  context: &'a Rc<TransformContext>,
  context_block: &'a mut BlockIRNode,
  parent_dynamic: &'a mut IRDynamicInfo,
) -> Result<Box<dyn FnOnce() -> Result<()> + 'a>> {
  let dynamic = &mut context_block.dynamic;
  dynamic.flags = dynamic.flags | DynamicFlag::NON_TEMPLATE as i32 | DynamicFlag::INSERT as i32;
  let id = context.reference(dynamic)?;
  let block = context_block as *mut BlockIRNode;
  let exit_block = context.create_block(
    unsafe { &mut *block },
    node.get_named_property::<Object>("consequent")?,
    None,
  )?;

  Ok(Box::new(move || {
    let block = exit_block()?;
    let test = node.get_named_property::<Object>("test")?;
    let alternate = node.get_named_property::<Object>("alternate")?;

    let mut operation = IfIRNode {
      _type: IRNodeTypes::IF,
      id,
      positive: block,
      once: Some(*context.in_v_once.borrow() || _is_constant_node(&Some(test))),
      condition: resolve_expression(test, context),
      negative: None,
      parent: None,
      anchor: None,
    };
    set_negative(
      alternate,
      &mut operation,
      context,
      context_block,
      parent_dynamic,
    )?;
    let dynamic = &mut context_block.dynamic;
    dynamic.operation = Some(MyBox(Box::new(Either18::A(operation))));

    Ok(())
  }))
}

fn process_logical_expression<'a>(
  node: Object<'static>,
  context: &'a Rc<TransformContext>,
  context_block: &'a mut BlockIRNode,
  parent_dynamic: &'a mut IRDynamicInfo,
) -> Result<Box<dyn FnOnce() -> Result<()> + 'a>> {
  let left = node.get_named_property::<Object>("left")?;
  let right = node.get_named_property::<Object>("right")?;
  let operator = node.get_named_property::<String>("operator")?;

  let dynamic = &mut context_block.dynamic;
  dynamic.flags = dynamic.flags | DynamicFlag::NON_TEMPLATE as i32 | DynamicFlag::INSERT as i32;
  let id = context.reference(dynamic)?;
  let block = context_block as *mut BlockIRNode;
  let exit_block = context.create_block(
    unsafe { &mut *block },
    if operator == "&&" { right } else { left },
    None,
  )?;
  Ok(Box::new(move || {
    let block = exit_block()?;
    let left = node.get_named_property::<Object>("left")?;
    let right = node.get_named_property::<Object>("right")?;
    let operator = node.get_named_property::<String>("operator")?;

    let mut operation = IfIRNode {
      _type: IRNodeTypes::IF,
      id,
      condition: resolve_expression(left, context),
      positive: block,
      once: Some(*context.in_v_once.borrow() || _is_constant_node(&Some(left))),
      negative: None,
      anchor: None,
      parent: None,
    };
    set_negative(
      if operator == "&&" { left } else { right },
      &mut operation,
      context,
      context_block,
      parent_dynamic,
    )?;
    let dynamic = &mut context_block.dynamic;
    dynamic.operation = Some(MyBox(Box::new(Either18::A(operation))));
    Ok(())
  }))
}

fn set_negative(
  node: Object<'static>,
  operation: &mut IfIRNode,
  context: &Rc<TransformContext>,
  context_block: &mut BlockIRNode,
  parent_dynamic: &mut IRDynamicInfo,
) -> Result<()> {
  let node_type = node.get_named_property::<String>("type")?;
  if node_type == "ConditionalExpression" {
    let block = context_block as *mut BlockIRNode;
    let exit_block = context.create_block(
      unsafe { &mut *block },
      node.get_named_property::<Object>("consequent")?,
      None,
    )?;
    let test = node.get_named_property::<Object>("test")?;
    context.transform_node(context_block, parent_dynamic)?;
    let block = exit_block()?;
    let mut negative = IfIRNode {
      _type: IRNodeTypes::IF,
      id: -1,
      condition: resolve_expression(test, context),
      positive: block,
      once: Some(*context.in_v_once.borrow() || _is_constant_node(&Some(test))),
      negative: None,
      anchor: None,
      parent: None,
    };
    set_negative(
      node.get_named_property::<Object>("alternate")?,
      &mut negative,
      context,
      context_block,
      parent_dynamic,
    )?;
    operation.negative = Some(MyBox(Box::new(Either::B(negative))));
  } else if node_type == "LogicalExpression" {
    let left = node.get_named_property::<Object>("left")?;
    let right = node.get_named_property::<Object>("right")?;
    let operator = node.get_named_property::<String>("operator")?;
    let block = context_block as *mut BlockIRNode;
    let exit_block = context.create_block(
      unsafe { &mut *block },
      if operator.eq("&&") { right } else { left },
      None,
    )?;
    context.transform_node(context_block, parent_dynamic)?;
    let block = exit_block()?;
    let mut negative = IfIRNode {
      _type: IRNodeTypes::IF,
      id: -1,
      condition: resolve_expression(left, context),
      positive: block,
      once: Some(*context.in_v_once.borrow() || _is_constant_node(&Some(left))),
      negative: None,
      anchor: None,
      parent: None,
    };
    set_negative(
      if operator.eq("&&") { left } else { right },
      &mut negative,
      context,
      context_block,
      parent_dynamic,
    )?;
    operation.negative = Some(MyBox(Box::new(Either::B(negative))));
  } else {
    let block = context_block as *mut BlockIRNode;
    let exit_block = context.create_block(unsafe { &mut *block }, node, None)?;
    context.transform_node(context_block, parent_dynamic)?;
    let block = exit_block()?;
    operation.negative = Some(MyBox(Box::new(Either::A(block))));
  }
  Ok(())
}
