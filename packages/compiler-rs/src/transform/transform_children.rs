use napi::{
  Env, Result,
  bindgen_prelude::{Either18, FnArgs, Function, JsObjectValue, Object},
};

use crate::{
  ir::index::{DynamicFlag, IRNodeTypes, InsertNodeIRNode, is_block_operation},
  transform::{reference, register_operation, register_template, transform_node},
  utils::check::{is_fragment_node, is_jsx_component},
};

pub fn transform_children(
  env: Env,
  node: Object<'static>,
  context: Object<'static>,
) -> Result<Option<Box<dyn FnOnce() -> Result<()>>>> {
  let node_type = node.get_named_property::<String>("type")?;
  let is_fragment_or_component = is_fragment_node(node) || is_jsx_component(node);

  if node_type != "JSXElement" && !is_fragment_or_component {
    return Ok(None);
  }

  let mut i = 0;
  let mut dynamic = context
    .get_named_property::<Object>("block")?
    .get_named_property::<Object>("dynamic")?;
  for child in node.get_named_property::<Vec<Object>>("children")? {
    let child_context = context
      .get_named_property::<Function<FnArgs<(Object, i32)>, Object>>("create")?
      .apply(context, FnArgs::from((child, i)))?;
    let mut child_dynamic = child_context
      .get_named_property::<Object>("block")?
      .get_named_property::<Object>("dynamic")?;
    child_dynamic.set("parent", dynamic)?;
    transform_node(env, child_context)?;

    let flags = child_dynamic.get_named_property::<i32>("flags")?;
    if is_fragment_or_component {
      reference(child_context)?;
      register_template(child_context)?;

      if flags & DynamicFlag::NON_TEMPLATE as i32 == 0 || flags & DynamicFlag::INSERT as i32 != 0 {
        let returns = context
          .get_named_property::<Object>("block")?
          .get_named_property::<Object>("returns")?;
        returns
          .get_named_property::<Function<i32, i32>>("push")?
          .apply(returns, child_dynamic.get_named_property::<i32>("id")?)?;
      }
    } else {
      let children_template = context.get_named_property::<Object>("childrenTemplate")?;
      children_template
        .get_named_property::<Function<String, i32>>("push")?
        .apply(
          children_template,
          child_context.get_named_property::<String>("template")?,
        )?;
    }

    if child_dynamic
      .get_named_property::<bool>("hasDynamicChild")
      .unwrap_or(false)
      || child_dynamic.get_named_property::<i32>("id").is_ok()
      || flags & DynamicFlag::NON_TEMPLATE as i32 != 0
      || flags & DynamicFlag::INSERT as i32 != 0
    {
      dynamic.set("hasDynamicChild", true)?;
    }

    dynamic.get_named_property::<Object>("children")?.set(
      i.to_string(),
      child_context
        .get_named_property::<Object>("block")?
        .get_named_property::<Object>("dynamic")?,
    )?;

    i = i + 1;
  }
  context
    .get_named_property::<Object>("block")?
    .set("dynamic", dynamic)?;

  if !is_fragment_or_component {
    process_dynamic_children(context)?;
  }

  Ok(None)
}

fn process_dynamic_children(context: Object) -> Result<()> {
  let mut prev_dynamics = vec![];
  let mut has_static_template = false;
  let children = context
    .get_named_property::<Object>("block")?
    .get_named_property::<Object>("dynamic")?
    .get_named_property::<Vec<Object>>("children")?;

  let mut index = 0;
  for child in children {
    let flags = child.get_named_property::<i32>("flags")?;
    if flags & DynamicFlag::INSERT as i32 != 0 {
      prev_dynamics.push(child);
    }

    if flags & DynamicFlag::NON_TEMPLATE as i32 == 0 {
      if prev_dynamics.len() > 0 {
        if has_static_template {
          context
            .get_named_property::<Object>("childrenTemplate")?
            .set((index - prev_dynamics.len()).to_string(), "<!>")?;
          let flags =
            prev_dynamics[0].get_named_property::<i32>("flags")? - DynamicFlag::NON_TEMPLATE as i32;
          prev_dynamics[0].set("flags", flags)?;
          let anchor = context
            .get_named_property::<Function<(), i32>>("increaseId")?
            .apply(context, ())?;
          prev_dynamics[0].set("anchor", anchor)?;
          register_insertion(&prev_dynamics, context, Some(anchor))?;
        } else {
          register_insertion(&prev_dynamics, context, Some(-1) /* prepend */)?;
        }
        prev_dynamics = vec![];
      }
      has_static_template = true;
    }
    index = index + 1
  }

  if prev_dynamics.len() > 0 {
    register_insertion(&prev_dynamics, context, None)?;
  }

  Ok(())
}

pub fn register_insertion(
  dynamics: &Vec<Object>,
  context: Object,
  anchor: Option<i32>,
) -> Result<()> {
  for child in dynamics {
    if child.get_named_property::<i32>("template").is_ok() {
      // template node due to invalid nesting - generate actual insertion
      register_operation(
        &context,
        Either18::L(InsertNodeIRNode {
          _type: IRNodeTypes::INSERT_NODE,
          elements: dynamics
            .iter()
            .map(|child| child.get_named_property::<i32>("id").unwrap())
            .collect(),
          parent: reference(context)?,
          anchor,
        }),
        None,
      )?;
    } else if let Ok(mut operation) = child.get_named_property("operation")
      && is_block_operation(operation)
    {
      // block types
      operation.set("parent", reference(context)?)?;
      operation.set("anchor", anchor)?;
    }
  }

  Ok(())
}
