use std::{collections::HashMap, rc::Rc};

use napi::{
  Either,
  bindgen_prelude::{Either3, Either4, JsObjectValue, Object, Result},
};

use crate::{
  ir::{
    component::{IRSlotDynamicBasic, IRSlotDynamicConditional, IRSlotType, IRSlots, IRSlotsStatic},
    index::{BlockIRNode, DirectiveNode, DynamicFlag, IRDynamicInfo, SimpleExpressionNode},
  },
  transform::{TransformContext, v_for::get_for_parse_result},
  utils::{
    check::{is_jsx_component, is_template},
    directive::resolve_directive,
    error::{ErrorCodes, on_error},
    my_box::MyBox,
    text::is_empty_text,
    utils::find_prop,
  },
};

pub fn transform_v_slot<'a>(
  node: Object<'static>,
  context: &'a Rc<TransformContext>,
  context_block: &'a mut BlockIRNode,
  _: &'a mut IRDynamicInfo,
) -> Result<Option<Box<dyn FnOnce() -> Result<()> + 'a>>> {
  if !node.get_named_property::<String>("type")?.eq("JSXElement") {
    return Ok(None);
  }

  let dir = find_prop(&node, Either::A(String::from("v-slot")))
    .map(|dir| resolve_directive(dir, context).unwrap());
  let is_component = is_jsx_component(node);
  let parent_node = *context.parent.borrow().upgrade().unwrap().node.borrow();
  let is_slot_template = is_template(&node)
    && parent_node
      .get_named_property::<String>("type")
      .unwrap()
      .eq("JSXElement")
    && is_jsx_component(parent_node);

  if is_component && node.get_named_property::<Vec<Object>>("children")?.len() > 0 {
    return Ok(Some(transform_component_slot(
      dir,
      node,
      context,
      context_block,
    )?));
  } else if is_slot_template && let Some(dir) = dir {
    return Ok(Some(transform_template_slot(
      dir,
      node,
      context,
      context_block,
    )?));
  } else if !is_component && dir.is_some() {
    on_error(ErrorCodes::X_V_SLOT_MISPLACED, context);
  }

  Ok(None)
}

// <Foo v-slot:default>
pub fn transform_component_slot<'a>(
  dir: Option<DirectiveNode>,
  node: Object<'static>,
  context: &'a Rc<TransformContext>,
  context_block: &'a mut BlockIRNode,
) -> Result<Box<dyn FnOnce() -> Result<()> + 'a>> {
  let children = node.get_named_property::<Vec<Object>>("children")?;
  let has_dir = dir.is_some();

  let non_slot_template_children: Vec<Object> = children
    .into_iter()
    .filter(|n| {
      !is_empty_text(n.to_owned())
        && (!n
          .get_named_property::<String>("type")
          .is_ok_and(|ty| ty.eq("JSXElement"))
          || find_prop(n, Either::A(String::from("v-slot"))).is_none())
    })
    .collect();

  let exit_block = create_slot_block(
    dir.map_or(None, |dir| dir.exp),
    node,
    context,
    context_block,
    false,
  )?;

  Ok(Box::new(move || {
    let dir = find_prop(&node, Either::A(String::from("v-slot")))
      .map(|dir| resolve_directive(dir, context).unwrap());
    let arg = dir.map_or(None, |dir| dir.arg);
    let mut slots = context.slots.take();

    let block = exit_block()?;
    let has_other_slots = !slots.is_empty();
    if has_dir && has_other_slots {
      // already has on-component slot - this is incorrect usage.
      on_error(ErrorCodes::X_V_SLOT_MIXED_SLOT_USAGE, context);
      return Ok(());
    }

    if !non_slot_template_children.is_empty() {
      if has_static_slot(&slots, "default") {
        on_error(
          ErrorCodes::X_V_SLOT_EXTRANEOUS_DEFAULT_SLOT_CHILDREN,
          context,
        );
      } else {
        register_slot(&mut slots, arg, block);
        *context.slots.borrow_mut() = slots;
      }
    } else if has_other_slots {
      *context.slots.borrow_mut() = slots
    }

    Ok(())
  }))
}

// <template v-slot:foo>
pub fn transform_template_slot<'a>(
  dir: DirectiveNode,
  node: Object<'static>,
  context: &'a Rc<TransformContext>,
  context_block: &'a mut BlockIRNode,
) -> Result<Box<dyn FnOnce() -> Result<()> + 'a>> {
  let dynamic = &mut context_block.dynamic;
  dynamic.flags |= DynamicFlag::NON_TEMPLATE as i32;

  let DirectiveNode { arg, exp, .. } = dir;
  let exit_block = create_slot_block(exp, node, context, context_block, true)?;

  Ok(Box::new(move || {
    let v_for = find_prop(&node, Either::A(String::from("v-for")));
    let v_if = find_prop(&node, Either::A(String::from("v-if")));
    let v_else = find_prop(
      &node,
      Either::B(vec![String::from("v-else"), String::from("v-else-if")]),
    );
    let slots = &mut context.slots.borrow_mut();
    let block = exit_block()?;
    if v_for.is_none() && v_if.is_none() && v_else.is_none() {
      let slot_name = if let Some(arg) = &arg {
        arg.content.clone()
      } else {
        String::from("default")
      };
      if !slot_name.is_empty() && has_static_slot(&slots, &slot_name) {
        on_error(ErrorCodes::X_V_SLOT_DUPLICATE_SLOT_NAMES, context)
      } else {
        register_slot(slots, arg, block);
      }
    } else if let Some(v_if) = v_if {
      let v_if_dir = resolve_directive(v_if, context)?;
      slots.push(Either4::C(IRSlotDynamicConditional {
        slot_type: IRSlotType::CONDITIONAL,
        condition: v_if_dir.exp.unwrap(),
        negative: None,
        positive: IRSlotDynamicBasic {
          slot_type: IRSlotType::DYNAMIC,
          name: arg.unwrap(),
          _fn: block,
          _loop: None,
        },
      }));
    } else if let Some(v_else) = v_else {
      let v_else_dir = resolve_directive(v_else, context)?;
      if let Some(last_slot) = slots.last_mut() {
        if let Either4::C(v_if_slot) = last_slot {
          let positive = IRSlotDynamicBasic {
            slot_type: IRSlotType::DYNAMIC,
            name: arg.unwrap(),
            _fn: block,
            _loop: None,
          };
          let negative = if let Some(exp) = v_else_dir.exp {
            Either::B(IRSlotDynamicConditional {
              slot_type: IRSlotType::CONDITIONAL,
              condition: exp,
              positive,
              negative: None,
            })
          } else {
            Either::A(positive)
          };
          set_slot(v_if_slot, negative);
        } else {
          on_error(ErrorCodes::X_V_ELSE_NO_ADJACENT_IF, context)
        }
      }
    } else if let Some(v_for) = v_for {
      let for_parse_result = get_for_parse_result(v_for, context)?;
      if for_parse_result.source.is_some() {
        slots.push(Either4::B(IRSlotDynamicBasic {
          slot_type: IRSlotType::DYNAMIC,
          name: arg.unwrap(),
          _fn: block,
          _loop: Some(for_parse_result),
        }))
      }
    }
    Ok(())
  }))
}

fn set_slot(
  v_if_slot: &mut IRSlotDynamicConditional,
  slot: Either<IRSlotDynamicBasic, IRSlotDynamicConditional>,
) {
  if let Some(Either::B(negative)) = v_if_slot.negative.as_mut().map(|a| a.0.as_mut()) {
    return set_slot(negative, slot);
  } else {
    v_if_slot.negative = Some(MyBox(Box::new(slot)));
  };
}

pub fn ensure_static_slots<'a>(
  slots: &'a mut Vec<IRSlots>,
) -> Option<&'a mut HashMap<String, BlockIRNode>> {
  let last_slot = slots.last();
  if slots.is_empty() || last_slot.is_some_and(|last_slot| !matches!(last_slot, Either4::A(_))) {
    slots.push(Either4::A(IRSlotsStatic {
      slot_type: IRSlotType::STATIC,
      slots: HashMap::new(),
    }));
  }
  let last_slot = slots.last_mut();
  if let Either4::A(slot) = last_slot.unwrap() {
    return Some(&mut slot.slots);
  }
  None
}

pub fn _ensure_static_slots<'a>(slots: &mut Object) -> Result<Object<'a>> {
  let len: i32 = slots.get_named_property("length")?;
  let last_index = if len > 0 { len - 1 } else { 0 };
  let last_slot = slots.get_named_property::<IRSlots>(last_index.to_string().as_str());
  if len == 0 || last_slot.is_ok_and(|last_slot| !matches!(last_slot, Either4::A(_))) {
    slots.set(
      (if len > 0 { len } else { 0 }).to_string(),
      IRSlotsStatic {
        slot_type: IRSlotType::STATIC,
        slots: HashMap::new(),
      },
    )?;
  }
  let last_slot = slots.get_named_property::<Object>(
    (slots.get_named_property::<i32>("length")? - 1)
      .to_string()
      .as_str(),
  )?;
  Ok(last_slot.get_named_property::<Object>("slots")?)
}

fn register_slot(slots: &mut Vec<IRSlots>, name: Option<SimpleExpressionNode>, block: BlockIRNode) {
  let is_static = if let Some(name) = &name {
    name.is_static
  } else {
    true
  };
  if is_static {
    let slots = ensure_static_slots(slots).unwrap();
    slots.insert(
      if let Some(name) = &name {
        name.content.clone()
      } else {
        String::from("default")
      },
      block,
    );
  } else {
    slots.push(Either4::B(IRSlotDynamicBasic {
      slot_type: IRSlotType::DYNAMIC,
      name: name.unwrap(),
      _fn: block,
      _loop: None,
    }));
  }
}

fn has_static_slot(slots: &Vec<IRSlots>, name: &str) -> bool {
  slots.iter().any(|slot| match slot {
    Either4::A(static_slot) => {
      matches!(static_slot.slot_type, IRSlotType::STATIC) && static_slot.slots.get(name).is_some()
    }
    _ => false,
  })
}

fn create_slot_block<'a>(
  props: Option<SimpleExpressionNode>,
  node: Object<'static>,
  context: &'a Rc<TransformContext>,
  context_block: &'a mut BlockIRNode,
  exclude_slots: bool,
) -> Result<Box<dyn FnOnce() -> Result<BlockIRNode> + 'a>> {
  let mut block = BlockIRNode::new(Some(node));
  block.props = props;
  let exit_block = context.enter_block(context_block, block, false, exclude_slots)?;
  Ok(exit_block)
}
