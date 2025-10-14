use std::collections::HashSet;

use napi::{
  Either, Env, Result,
  bindgen_prelude::{Function, JsObjectValue, Object},
};
use napi_derive::napi;

use crate::{
  ir::index::{DynamicFlag, ForIRNode, IRFor, IRNodeTypes, SimpleExpressionNode},
  utils::{
    check::{_is_constant_node, is_jsx_component, is_template},
    error::{ErrorCodes, on_error},
    expression::resolve_expression,
    text::is_empty_text,
    transform::create_branch,
    utils::{find_prop, get_expression},
  },
};

#[napi]
pub fn transform_v_for<'a>(
  env: &'a Env,
  node: Object<'static>,
  context: Object,
) -> Result<Option<Function<'a, (), ()>>> {
  if !node.get_named_property::<String>("type")?.eq("JSXElement")
    || (is_template(Some(node)) && find_prop(node, Either::A("v-slot".to_string())).is_some())
  {
    return Ok(None);
  }
  let Some(dir) = find_prop(node, Either::A("v-for".to_string())) else {
    return Ok(None);
  };
  let seen = context.get_named_property::<HashSet<i32>>("seen")?;
  let dir_start = dir.get_named_property::<i32>("start")?;
  if seen.contains(&dir_start) {
    return Ok(None);
  }
  let seen = context.get_named_property::<Object>("seen")?;
  seen
    .get_named_property::<Function<i32>>("add")?
    .apply(seen, dir_start)?;

  let component = is_jsx_component(node) || is_template_with_single_component(node)?;
  let id = context
    .get_named_property::<Function<(), i32>>("reference")?
    .apply(context, ())?;
  let mut dynamic = context.get_named_property::<Object>("dynamic")?;
  dynamic.set(
    "flags",
    dynamic.get_named_property::<i32>("flags")?
      | DynamicFlag::NON_TEMPLATE as i32
      | DynamicFlag::INSERT as i32,
  )?;
  let (.., exit_key) = create_branch(*env, node, context, Some(true))?;
  context
    .get_named_property::<Object>("nodes")?
    .set(&exit_key, node)?;
  Ok(Some(env.create_function_from_closure("cb", move |e| {
    let context = e.first_arg::<Object>()?;

    context
      .get_named_property::<Object>("exitBlocks")?
      .get_named_property::<Function<(), ()>>(&exit_key)?
      .apply(&context, ())?;
    let parent = context.get_named_property::<Object>("parent");
    let node = context
      .get_named_property::<Object>("nodes")?
      .get_named_property::<Object>(&exit_key)?;
    let Some(dir) = find_prop(node, Either::A("v-for".to_string())) else {
      return Ok(());
    };
    let IRFor {
      value,
      index,
      key,
      source,
    } = get_for_parse_result(e.env, dir, context)?;
    let Some(source) = source else {
      on_error(*e.env, ErrorCodes::X_V_FOR_MALFORMED_EXPRESSION, context);
      return Ok(());
    };

    let key_prop = find_prop(node, Either::A("key".to_string()));
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
    let only_child = !e.env.strict_equals(
      context
        .get_named_property::<Object>("parent")?
        .get_named_property::<Object>("block")?
        .get_named_property::<Object>("node")?,
      context
        .get_named_property::<Object>("parent")?
        .get_named_property::<Object>("node")?,
    )? && parent?
      .get_named_property::<Object>("node")?
      .get_named_property::<Vec<Object>>("children")?
      .into_iter()
      .filter(|child| !is_empty_text(*child))
      .collect::<Vec<Object>>()
      .len()
      == 1;

    context.get_named_property::<Object>("dynamic")?.set(
      "operation",
      ForIRNode {
        _type: IRNodeTypes::FOR,
        id,
        value,
        key,
        index,
        key_prop,
        render: context
          .get_named_property::<Object>("blocks")?
          .get_named_property::<Object>(&exit_key)?,
        once: context.get_named_property::<bool>("inVOnce")? || _is_constant_node(&source.ast),
        source,
        component,
        only_child,
        parent: None,
        anchor: None,
      },
    )?;

    Ok(())
  })?))
}

pub fn get_for_parse_result(env: &Env, dir: Object, context: Object) -> Result<IRFor> {
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
    on_error(*env, ErrorCodes::X_V_FOR_NO_EXPRESSION, context);
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
