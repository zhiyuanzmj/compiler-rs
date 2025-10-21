use std::{mem, rc::Rc};

use napi::{
  Result,
  bindgen_prelude::{Either18, JsObjectValue, Object},
};

use crate::{
  ir::index::{
    BlockIRNode, DynamicFlag, IRDynamicInfo, IRNodeTypes, InsertNodeIRNode, is_block_operation,
  },
  transform::TransformContext,
  utils::{
    check::{is_fragment_node, is_jsx_component},
    my_box::MyBox,
    text::get_text,
  },
};

pub fn transform_children<'a>(
  node: Object<'static>,
  context: &'a Rc<TransformContext>,
  context_block: &'a mut BlockIRNode,
  _: &'a mut IRDynamicInfo,
) -> Result<Option<Box<dyn FnOnce() -> Result<()> + 'a>>> {
  let node_type = node.get_named_property::<String>("type")?;
  let is_fragment_or_component = is_fragment_node(&node) || is_jsx_component(node);

  if node_type != "JSXElement" && !is_fragment_or_component {
    return Ok(None);
  }

  let mut i = 0;
  let mut dynamic = mem::take(&mut context_block.dynamic);
  let mut returns = mem::take(&mut context_block.returns);
  let children = node.get_named_property::<Vec<Object>>("children")?;
  for child in children {
    let child_context = context.create(child, i, context_block);
    let child_block = &mut child_context.block.borrow_mut();
    // TODO: child_dynamic.set("parent", dynamic)?;
    child_context.transform_node(child_block, &mut dynamic)?;

    let child_dynamic = &mut child_block.dynamic;
    let flags = child_dynamic.flags.clone();
    if is_fragment_or_component {
      child_context.register_template(child_dynamic)?;
      child_context.reference(child_dynamic)?;

      if flags & DynamicFlag::NON_TEMPLATE as i32 == 0 || flags & DynamicFlag::INSERT as i32 != 0 {
        returns.push(child_dynamic.id.unwrap());
      }
    } else {
      context
        .children_template
        .borrow_mut()
        .push(child_context.template.borrow().to_string());
    }

    if child_dynamic.has_dynamic_child.unwrap_or(false)
      || child_dynamic.id.is_some()
      || flags & DynamicFlag::NON_TEMPLATE as i32 != 0
      || flags & DynamicFlag::INSERT as i32 != 0
    {
      dynamic.has_dynamic_child = Some(true);
    }

    dynamic
      .children
      .insert(i as usize, mem::take(child_dynamic));

    i = i + 1;
    *context_block = mem::take(child_block);
  }
  context_block.dynamic = dynamic;
  context_block.returns = returns;

  if !is_fragment_or_component {
    process_dynamic_children(context, context_block)?;
  }

  Ok(None)
}

fn process_dynamic_children(
  context: &Rc<TransformContext>,
  context_block: &mut BlockIRNode,
) -> Result<()> {
  let mut prev_dynamics = vec![];
  let mut has_static_template = false;

  let mut index = 0;
  let children = &mut context_block.dynamic.children as *mut Vec<IRDynamicInfo>;
  for child in unsafe { &mut *children } {
    let flags = child.flags;
    if flags & DynamicFlag::INSERT as i32 != 0 {
      prev_dynamics.push(child);
    }

    if flags & DynamicFlag::NON_TEMPLATE as i32 == 0 {
      if prev_dynamics.len() > 0 {
        if has_static_template {
          context
            .children_template
            .borrow_mut()
            .insert(index - prev_dynamics.len(), "<!>".to_string());
          prev_dynamics[0].flags = prev_dynamics[0].flags - DynamicFlag::NON_TEMPLATE as i32;
          let anchor = context.increase_id()?;
          prev_dynamics[0].anchor = Some(anchor);
          register_insertion(&mut prev_dynamics, context, context_block, Some(anchor))?;
        } else {
          register_insertion(
            &mut prev_dynamics,
            context,
            context_block,
            Some(-1), /* prepend */
          )?;
        }
        prev_dynamics.clear();
      }
      has_static_template = true;
    }
    index = index + 1
  }

  if prev_dynamics.len() > 0 {
    register_insertion(&mut prev_dynamics, context, context_block, None)?;
  }

  Ok(())
}

pub fn register_insertion(
  dynamics: &mut Vec<&mut IRDynamicInfo>,
  context: &Rc<TransformContext>,
  context_block: &mut BlockIRNode,
  anchor: Option<i32>,
) -> Result<()> {
  let ids = dynamics
    .iter()
    .filter_map(|child| child.id)
    .collect::<Vec<i32>>();
  for child in dynamics {
    if child.template.is_some() {
      let parent = context.reference(&mut context_block.dynamic)?;
      // template node due to invalid nesting - generate actual insertion
      context.register_operation(
        context_block,
        Either18::L(InsertNodeIRNode {
          _type: IRNodeTypes::INSERT_NODE,
          elements: ids.clone(),
          parent,
          anchor,
        }),
        None,
      )?;
    } else if let Some(MyBox(operation)) = &mut child.operation
      && is_block_operation(&operation)
    {
      // block types
      let parent = context.reference(&mut context_block.dynamic)?;
      match operation.as_mut() {
        Either18::A(a) => {
          a.parent = Some(parent);
          a.anchor = anchor;
        }
        Either18::B(b) => {
          b.parent = Some(parent);
          b.anchor = anchor;
        }
        Either18::O(c) => {
          c.parent = Some(parent);
          c.anchor = anchor;
        }
        _ => (),
      };
    }
  }

  Ok(())
}
