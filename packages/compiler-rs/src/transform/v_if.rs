use std::collections::HashSet;

use napi::{
  Either, Env, Result,
  bindgen_prelude::{Function, JsObjectValue, Object},
};

use crate::{
  ir::index::{DynamicFlag, IRNodeTypes, IfIRNode},
  transform::reference,
  utils::{
    check::{_is_constant_node, is_template},
    directive::resolve_directive,
    error::{ErrorCodes, on_error},
    expression::create_simple_expression,
    transform::create_block,
    utils::find_prop,
  },
};

pub fn transform_v_if(
  env: Env,
  node: Object<'static>,
  context: Object<'static>,
) -> Result<Option<Box<dyn FnOnce() -> Result<()>>>> {
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
    on_error(env, ErrorCodes::X_V_IF_NO_EXPRESSION, context);
    dir.exp = Some(create_simple_expression(
      "true".to_string(),
      Some(false),
      None,
      None,
    ));
  }

  let mut dynamic = context
    .get_named_property::<Object>("block")?
    .get_named_property::<Object>("dynamic")?;
  dynamic.set(
    "flags",
    dynamic.get_named_property::<i32>("flags")? | DynamicFlag::NON_TEMPLATE as i32,
  )?;

  if dir.name == "if" {
    let id = reference(context)?;
    dynamic.set(
      "flags",
      dynamic.get_named_property::<i32>("flags")? | DynamicFlag::INSERT as i32,
    )?;
    let (block, exit_block) = create_block(env, node, context, None)?;
    return Ok(Some(Box::new(move || {
      exit_block()?;
      let dir = resolve_directive(
        find_prop(node, Either::A("v-if".to_string())).unwrap(),
        context,
      )?;

      context
        .get_named_property::<Object>("block")?
        .get_named_property::<Object>("dynamic")?
        .set(
          "operation",
          IfIRNode {
            _type: IRNodeTypes::IF,
            id,
            positive: block,
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
    })));
  }

  let siblings = dynamic
    .get_named_property::<Object>("parent")?
    .get_named_property::<Vec<Object>>("children");
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
    on_error(env, ErrorCodes::X_V_ELSE_NO_ADJACENT_IF, context);
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
    on_error(env, ErrorCodes::X_V_ELSE_NO_ADJACENT_IF, context);
  }

  let (branch, exit_block) = create_block(env, node, context, None)?;

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
  Ok(Some(exit_block))
}
