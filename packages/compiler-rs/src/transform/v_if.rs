use std::collections::HashSet;

use napi::{
  Either, Env, Result,
  bindgen_prelude::{Function, JsObjectValue, Object},
};
use napi_derive::napi;

use crate::{
  ir::index::{DynamicFlag, IRNodeTypes, IfIRNode},
  utils::{
    check::{_is_constant_node, is_template},
    directive::resolve_directive,
    error::{ErrorCodes, on_error},
    expression::create_simple_expression,
    transform::create_branch,
    utils::find_prop,
  },
};

#[napi]
pub fn transform_v_if<'a>(
  env: &'a Env,
  node: Object<'static>,
  context: Object,
) -> Result<Option<Function<'a, (), ()>>> {
  if !node.get_named_property::<String>("type")?.eq("JSXElement")
    || (is_template(Some(node)) && find_prop(node, Either::A("v-slot".to_string())).is_some())
  {
    return Ok(None);
  }
  let Some(prop) = find_prop(
    node,
    Either::B(vec![
      "v-if".to_string(),
      "v-else".to_string(),
      "v-else-if".to_string(),
    ]),
  ) else {
    return Ok(None);
  };
  let seen = context.get_named_property::<HashSet<i32>>("seen")?;
  let start = prop.get_named_property::<i32>("start")?;
  if seen.contains(&start) {
    return Ok(None);
  }
  let seen = context.get_named_property::<Object>("seen")?;
  seen
    .get_named_property::<Function<i32>>("add")?
    .apply(seen, start)?;

  let mut dir = resolve_directive(prop, context)?;
  if dir.name != "else"
    && (dir.exp.is_none() || dir.exp.as_ref().unwrap().content.trim().is_empty())
  {
    on_error(*env, ErrorCodes::X_V_IF_NO_EXPRESSION, context);
    dir.exp = Some(create_simple_expression(
      "true".to_string(),
      Some(false),
      None,
      None,
    ));
  }

  let mut dynamic = context.get_named_property::<Object>("dynamic")?;
  dynamic.set(
    "flags",
    dynamic.get_named_property::<i32>("flags")? | DynamicFlag::NON_TEMPLATE as i32,
  )?;

  if dir.name == "if" {
    let id = context
      .get_named_property::<Function<(), i32>>("reference")?
      .apply(context, ())?;
    dynamic.set(
      "flags",
      dynamic.get_named_property::<i32>("flags")? | DynamicFlag::INSERT as i32,
    )?;
    let (.., exit_key) = create_branch(*env, node, context, None)?;
    context
      .get_named_property::<Object>("nodes")?
      .set(&exit_key, node)?;
    return Ok(Some(env.create_function_from_closure("cb", move |e| {
      let context = e.get::<Object>(0)?;
      context
        .get_named_property::<Object>("exitBlocks")?
        .get_named_property::<Function<(), ()>>(&exit_key)?
        .apply(context, ())?;
      let node = context
        .get_named_property::<Object>("nodes")?
        .get_named_property::<Object>(&exit_key)?;
      let dir = resolve_directive(
        find_prop(node, Either::A("v-if".to_string())).unwrap(),
        context,
      )?;

      context.get_named_property::<Object>("dynamic")?.set(
        "operation",
        IfIRNode {
          _type: IRNodeTypes::IF,
          id,
          positive: context
            .get_named_property::<Object>("blocks")?
            .get_named_property::<Object>(&exit_key)?,
          once: Some(
            context.get_named_property::<bool>("inVOnce")?
              || _is_constant_node(&dir.exp.as_ref().unwrap().ast),
          ),
          condition: dir.exp.unwrap(),
          negative: None,
          anchor: None,
          parent: None,
        },
      )?;

      Ok(())
    })?));
  }

  let parent = context.get_named_property::<Object>("parent");
  let siblings = parent.map(|parent| {
    parent
      .get_named_property::<Object>("dynamic")
      .unwrap()
      .get_named_property::<Vec<Object>>("children")
      .unwrap()
  });
  let mut last_if_node = None;
  if let Ok(siblings) = siblings {
    let mut i = siblings.len();
    while i > 0 {
      i = i - 1;
      if let Ok(operation) = siblings[i].get_named_property::<Object>("operation")
        && operation.get_named_property::<String>("type")?.eq("IF")
      {
        last_if_node = Some(operation);
        break;
      }
    }
  }

  // check if IfNode is the last operation and get the root IfNode
  if if let Some(last_if_node) = last_if_node {
    !last_if_node.get_named_property::<String>("type")?.eq("IF")
  } else {
    true
  } {
    on_error(*env, ErrorCodes::X_V_ELSE_NO_ADJACENT_IF, context);
    return Ok(None);
  }

  let mut last_if_node = last_if_node.unwrap();
  while let Ok(negative) = last_if_node.get_named_property::<Object>("negative")
    && negative.get_named_property::<String>("type")?.eq("IF")
  {
    last_if_node = last_if_node.get_named_property::<Object>("negative")?;
  }

  // Check if v-else was followed by v-else-if
  if dir.name == "else-if"
    && last_if_node
      .get_named_property::<Object>("negative")
      .is_ok()
  {
    on_error(*env, ErrorCodes::X_V_ELSE_NO_ADJACENT_IF, context);
  }

  let (branch, on_exit, ..) = create_branch(*env, node, context, None)?;

  if dir.name == "else" {
    last_if_node.set("negative", branch)?;
  } else {
    last_if_node.set(
      "negative",
      IfIRNode {
        _type: IRNodeTypes::IF,
        id: -1,
        positive: branch,
        once: Some(
          context.get_named_property::<bool>("inVOnce")?
            || _is_constant_node(&dir.exp.as_ref().unwrap().ast),
        ),
        condition: dir.exp.unwrap(),
        parent: None,
        anchor: None,
        negative: None,
      },
    )?
  }
  Ok(Some(on_exit))
}
