use std::collections::HashSet;

use napi::{
  Either, Env,
  bindgen_prelude::{Either18, Function, JsObjectValue, Object, Result},
};

use crate::{
  ir::index::{
    CreateNodesIRNode, DynamicFlag, GetTextChildIRNode, IRNodeTypes, IfIRNode, SetNodesIRNode,
    SimpleExpressionNode,
  },
  transform::{reference, register_operation, transform_node},
  utils::{
    check::{_is_constant_node, is_fragment_node, is_jsx_component, is_template},
    expression::{_get_literal_expression_value, resolve_expression},
    text::{is_empty_text, resolve_jsx_text},
    transform::{_create_branch, create_branch},
    utils::{_get_expression, find_prop, get_expression},
  },
};

pub fn transform_text(
  env: Env,
  node: Object<'static>,
  mut context: Object<'static>,
) -> Result<Option<Box<dyn FnOnce() -> Result<()>>>> {
  let mut dynamic = context
    .get_named_property::<Object>("block")?
    .get_named_property::<Object>("dynamic")?;
  if let Ok(start) = node.get_named_property::<i32>("start") {
    let seen = context.get_named_property::<HashSet<i32>>("seen")?;
    if seen.contains(&start) {
      dynamic.set(
        "flags",
        dynamic.get_named_property::<i32>("flags")? | DynamicFlag::NON_TEMPLATE as i32,
      )?;
      return Ok(None);
    }
  }

  let children = node.get_named_property::<Vec<Object<'static>>>("children");
  let is_fragment = is_fragment_node(node);
  let node_type = node.get_named_property::<String>("type")?;
  if ((node_type.eq("JSXElement") && !is_template(Some(node)) && !is_jsx_component(node))
    || is_fragment)
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
      process_text_container(children, context)?
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
        env, expression, context,
      )?));
    } else if expression_type.eq("LogicalExpression") {
      return Ok(Some(process_logical_expression(env, expression, context)?));
    } else {
      precess_interpolation(context)?;
    }
  } else if node_type == "JSXText" {
    let value = resolve_jsx_text(node);
    if !value.is_empty() {
      context.set(
        "template",
        context.get_named_property::<String>("template")? + &value,
      )?;
    } else {
      dynamic.set(
        "flags",
        dynamic.get_named_property::<i32>("flags")? | DynamicFlag::NON_TEMPLATE as i32,
      )?;
    }
  }
  Ok(None)
}

fn precess_interpolation(mut context: Object) -> Result<()> {
  let parent = context
    .get_named_property::<Object>("parent")?
    .get_named_property::<Object>("node")?;
  let children = parent.get_named_property::<Vec<Object>>("children")?;
  let index = context.get_named_property::<i32>("index")? as usize;
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

  let values = precess_text_like_expressions(nodes, context)?;
  if values.is_empty() {
    let mut dynamic = context
      .get_named_property::<Object>("block")?
      .get_named_property::<Object>("dynamic")?;
    dynamic.set(
      "flags",
      dynamic.get_named_property::<i32>("flags")? | DynamicFlag::NON_TEMPLATE as i32,
    )?;
    return Ok(());
  }

  let id = reference(context)?;
  let once = context.get_named_property::<bool>("inVOnce")?;
  if is_fragment_node(parent) || find_prop(parent, Either::A(String::from("v-slot"))).is_some() {
    register_operation(
      &context,
      Either18::K(CreateNodesIRNode {
        _type: IRNodeTypes::CREATE_NODES,
        id,
        once,
        values,
      }),
      None,
    )?;
  } else {
    context.set(
      "template",
      context.get_named_property::<String>("template")? + " ",
    )?;
    register_operation(
      &context,
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

fn mark_non_template(node: Object, context: Object) -> Result<()> {
  let seen = context.get_named_property::<Object>("seen")?;
  seen
    .get_named_property::<Function<i32>>("add")?
    .apply(seen, node.get_named_property::<i32>("start")?)?;
  Ok(())
}

fn process_text_container(children: Vec<Object<'static>>, mut context: Object) -> Result<()> {
  let values = precess_text_like_expressions(children, context)?;
  let literals = values
    .iter()
    .map(_get_literal_expression_value)
    .collect::<Vec<Option<String>>>();
  if literals.iter().all(|l| l.is_some()) {
    context.set("childrenTemplate", literals)?;
  } else {
    context.set("childrenTemplate", vec![" ".to_string()])?;
    register_operation(
      &context,
      Either18::R(GetTextChildIRNode {
        _type: IRNodeTypes::GET_TEXT_CHILD,
        parent: reference(context)?,
      }),
      None,
    )?;
    register_operation(
      &context,
      Either18::G(SetNodesIRNode {
        _type: IRNodeTypes::SET_NODES,
        element: reference(context)?,
        once: context.get_named_property::<bool>("inVOnce")?,
        values,
        // indicates this node is generated, so prefix should be "x" instead of "n"
        generated: Some(true),
      }),
      None,
    )?;
  }
  Ok(())
}

fn precess_text_like_expressions(
  nodes: Vec<Object<'static>>,
  context: Object,
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

pub fn process_conditional_expression(
  env: Env,
  node: Object<'static>,
  context: Object<'static>,
) -> Result<Box<dyn FnOnce() -> Result<()>>> {
  let mut dynamic = context
    .get_named_property::<Object>("block")?
    .get_named_property::<Object>("dynamic")?;
  dynamic.set(
    "flags",
    dynamic.get_named_property::<i32>("flags")?
      | DynamicFlag::NON_TEMPLATE as i32
      | DynamicFlag::INSERT as i32,
  )?;
  let id = reference(context)?;
  let (block, exit_block) = _create_branch(
    env,
    node.get_named_property::<Object>("consequent")?,
    context,
    None,
  )?;

  Ok(Box::new(move || {
    exit_block()?;
    let test = node.get_named_property::<Object>("test")?;
    let alternate = node.get_named_property::<Object>("alternate")?;

    let mut dynamic = context
      .get_named_property::<Object>("block")?
      .get_named_property::<Object>("dynamic")?;
    dynamic.set(
      "operation",
      IfIRNode {
        _type: IRNodeTypes::IF,
        id,
        positive: block,
        once: Some(
          context
            .get_named_property::<bool>("inVOnce")
            .unwrap_or(false)
            || _is_constant_node(&Some(test)),
        ),
        condition: resolve_expression(test, context),
        negative: None,
        parent: None,
        anchor: None,
      },
    )?;

    set_negative(
      env,
      alternate,
      dynamic.get_named_property::<Object>("operation")?,
      context,
    )?;

    Ok(())
  }))
}

pub fn process_logical_expression(
  env: Env,
  node: Object<'static>,
  context: Object<'static>,
) -> Result<Box<dyn FnOnce() -> Result<()>>> {
  let left = node.get_named_property::<Object>("left")?;
  let right = node.get_named_property::<Object>("right")?;
  let operator = node.get_named_property::<String>("operator")?;

  let mut dynamic = context
    .get_named_property::<Object>("block")?
    .get_named_property::<Object>("dynamic")?;
  dynamic.set(
    "flags",
    dynamic.get_named_property::<i32>("flags")?
      | DynamicFlag::NON_TEMPLATE as i32
      | DynamicFlag::INSERT as i32,
  )?;
  let id = reference(context)?;
  let (block, exit_block) = _create_branch(
    env,
    if operator == "&&" { right } else { left },
    context,
    None,
  )?;
  Ok(Box::new(move || {
    exit_block()?;
    let left = node.get_named_property::<Object>("left")?;
    let right = node.get_named_property::<Object>("right")?;
    let operator = node.get_named_property::<String>("operator")?;

    let operation = IfIRNode {
      _type: IRNodeTypes::IF,
      id,
      condition: resolve_expression(left, context),
      positive: block,
      once: Some(context.get_named_property::<bool>("inVOnce")? || _is_constant_node(&Some(left))),
      negative: None,
      anchor: None,
      parent: None,
    };
    context
      .get_named_property::<Object>("block")?
      .get_named_property::<Object>("dynamic")?
      .set("operation", operation)?;
    set_negative(
      env,
      if operator == "&&" { left } else { right },
      context
        .get_named_property::<Object>("block")?
        .get_named_property::<Object>("dynamic")?
        .get_named_property::<Object>("operation")?,
      context,
    )?;
    Ok(())
  }))
}

pub fn set_negative(
  env: Env,
  node: Object<'static>,
  mut operation: Object,
  context: Object<'static>,
) -> Result<()> {
  let node_type = node.get_named_property::<String>("type")?;
  if node_type == "ConditionalExpression" {
    let (branch, on_exit, _) = create_branch(
      env,
      node.get_named_property::<Object>("consequent")?,
      context,
      None,
    )?;
    let test = node.get_named_property::<Object>("test")?;
    let negative = IfIRNode {
      _type: IRNodeTypes::IF,
      id: -1,
      condition: resolve_expression(test, context),
      positive: branch,
      once: Some(context.get_named_property::<bool>("inVOnce")? || _is_constant_node(&Some(test))),
      negative: None,
      anchor: None,
      parent: None,
    };
    operation.set("negative", negative)?;
    transform_node(env, context)?;
    set_negative(
      env,
      node.get_named_property::<Object>("alternate")?,
      operation.get_named_property::<Object>("negative")?,
      context,
    )?;
    on_exit.call(())?;
  } else if node_type == "LogicalExpression" {
    let left = node.get_named_property::<Object>("left")?;
    let right = node.get_named_property::<Object>("right")?;
    let operator = node.get_named_property::<String>("operator")?;
    let (branch, on_exit, ..) = create_branch(
      env,
      if operator.eq("&&") { right } else { left },
      context,
      None,
    )?;
    let negative = IfIRNode {
      _type: IRNodeTypes::IF,
      id: -1,
      condition: resolve_expression(left, context),
      positive: branch,
      once: Some(context.get_named_property::<bool>("inVOnce")? || _is_constant_node(&Some(left))),
      negative: None,
      anchor: None,
      parent: None,
    };
    operation.set("negative", negative)?;
    transform_node(env, context)?;
    set_negative(
      env,
      if operator.eq("&&") { left } else { right },
      operation.get_named_property::<Object>("negative")?,
      context,
    )?;
    on_exit.call(())?;
  } else {
    let (branch, on_exit, ..) = create_branch(env, node, context, None)?;
    operation.set("negative", branch)?;
    transform_node(env, context)?;
    on_exit.call(())?;
  }
  Ok(())
}
