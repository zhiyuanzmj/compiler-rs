use std::{
  cell::RefCell,
  collections::HashMap,
  ops::{Deref, DerefMut, RangeTo},
  rc::Rc,
};

use napi::{
  Either, Env, Unknown, ValueType,
  bindgen_prelude::{Either3, FnArgs, Function, JsObjectValue, Object, Result, ToNapiValue},
};
use napi_derive::napi;
use oxc_allocator::IntoIn;

use crate::{
  ir::{
    component::{
      IRSlotDynamicBasic, IRSlotDynamicConditional, IRSlotDynamicLoop, IRSlotType, IRSlots,
      IRSlotsStatic, SlotBlockIRNode,
    },
    index::{
      BlockIRNode, DirectiveNode, DynamicFlag, IRDynamicInfo, IRNodeTypes, SimpleExpressionNode,
    },
  },
  transform::{enter_block, v_for::get_for_parse_result},
  utils::{
    check::{is_jsx_component, is_template},
    directive::resolve_directive,
    error::{ErrorCodes, on_error},
    my_box::MyBox,
    text::is_empty_text,
    utils::find_prop,
  },
};

pub fn transform_v_slot(
  env: Env,
  node: Object<'static>,
  context: Object<'static>,
) -> Result<Option<Box<dyn FnOnce() -> Result<()>>>> {
  if !node.get_named_property::<String>("type")?.eq("JSXElement") {
    return Ok(None);
  }

  let dir = find_prop(node, Either::A(String::from("v-slot")))
    .map(|dir| resolve_directive(dir, context).unwrap());
  let is_component = is_jsx_component(node);
  let is_slot_template = is_template(Some(node))
    && context
      .get_named_property::<Object>("parent")?
      .get_named_property::<Object>("node")
      .is_ok_and(|node| {
        node
          .get_named_property::<String>("type")
          .unwrap()
          .eq("JSXElement")
          && is_jsx_component(node)
      });

  if is_component && node.get_named_property::<Vec<Object>>("children")?.len() > 0 {
    return Ok(Some(transform_component_slot(env, dir, node, context)?));
  } else if is_slot_template && let Some(dir) = dir {
    return Ok(Some(transform_template_slot(env, dir, node, context)?));
  } else if !is_component && dir.is_some() {
    on_error(env, ErrorCodes::X_V_SLOT_MISPLACED, context);
  }

  Ok(None)
}

// <Foo v-slot:default>
pub fn transform_component_slot(
  env: Env,
  dir: Option<DirectiveNode>,
  node: Object<'static>,
  mut context: Object<'static>,
) -> Result<Box<dyn FnOnce() -> Result<()>>> {
  let children = node.get_named_property::<Vec<Object>>("children")?;
  let has_dir = dir.is_some();

  let non_slot_template_children: Vec<Object> = children
    .into_iter()
    .filter(|n| {
      !is_empty_text(n.to_owned())
        && (!n
          .get_named_property::<String>("type")
          .is_ok_and(|ty| ty.eq("JSXElement"))
          || find_prop(n.to_owned(), Either::A(String::from("v-slot"))).is_none())
    })
    .collect();

  let (.., exit_key) = _create_slot_block(dir.map_or(None, |dir| dir.exp), node, context, false)?;

  Ok(Box::new(move || {
    let dir = find_prop(node, Either::A(String::from("v-slot")))
      .map(|dir| resolve_directive(dir, context).unwrap());
    let arg = dir.map_or(None, |dir| dir.arg);
    let block = context.get_named_property::<SlotBlockIRNode>("block")?;
    let block1 = context.get_named_property::<Object>("block")?;
    let mut slots = context.get_named_property::<Vec<IRSlots>>("slots")?;
    // let mut _slots = context.get_named_property::<Object>("slots")?;

    context
      .get_named_property::<Object>("exitBlocks")?
      .get_named_property::<Function<(), ()>>(&exit_key)?
      .apply(&context, ())?;
    // exit_block()?;
    let has_other_slots = !slots.is_empty();
    if has_dir && has_other_slots {
      // already has on-component slot - this is incorrect usage.
      on_error(env, ErrorCodes::X_V_SLOT_MIXED_SLOT_USAGE, context);
      return Ok(());
    }

    if !non_slot_template_children.is_empty() {
      if has_static_slot(&slots, "default") {
        on_error(
          env,
          ErrorCodes::X_V_SLOT_EXTRANEOUS_DEFAULT_SLOT_CHILDREN,
          context,
        );
      } else {
        register_slot(&mut slots, arg, block, block1);
        context.set("slots", slots)?;
        // _register_slot(_slots, arg, block);
        // context.set("slots", _slots)?;
      }
    } else if has_other_slots {
      context.set("slots", slots)?;
    }

    Ok(())
  }))
}

// <template v-slot:foo>
pub fn transform_template_slot(
  env: Env,
  dir: DirectiveNode,
  node: Object<'static>,
  context: Object<'static>,
) -> Result<Box<dyn FnOnce() -> Result<()>>> {
  let mut dynamic = context
    .get_named_property::<Object>("block")?
    .get_named_property::<Object>("dynamic")?;
  dynamic.set(
    "flags",
    dynamic.get_named_property::<i32>("flags")? | DynamicFlag::NON_TEMPLATE as i32,
  )?;

  let DirectiveNode { arg, exp, .. } = dir;
  let v_for = find_prop(node, Either::A(String::from("v-for")));
  let v_if = find_prop(node, Either::A(String::from("v-if")));
  let v_else = find_prop(
    node,
    Either::B(vec![String::from("v-else"), String::from("v-else-if")]),
  );
  let slots = context.get_named_property::<Object>("slots")?;
  let mut _slots = context.get_named_property::<Vec<IRSlots>>("slots")?;
  let (block, exit_block, ..) = _create_slot_block(exp, node, context, true)?;

  let is_basic = v_for.is_none() && v_if.is_none() && v_else.is_none();

  if is_basic {
    let slot_name = if let Some(arg) = &arg {
      arg.content.clone()
    } else {
      String::from("default")
    };
    if !slot_name.is_empty() && has_static_slot(&_slots, &slot_name) {
      on_error(env, ErrorCodes::X_V_SLOT_DUPLICATE_SLOT_NAMES, context)
    } else {
      _register_slot(slots, arg, block);
    }
  } else if let Some(v_if) = v_if {
    let v_if_dir = resolve_directive(v_if, context)?;
    _slots.push(Either3::B(Either3::C(
      // slots.set(
      //   slots.get_named_property::<i32>("length")?.to_string(),
      IRSlotDynamicConditional {
        slot_type: IRSlotType::CONDITIONAL,
        condition: v_if_dir.exp.unwrap(),
        negative: None,
        positive: IRSlotDynamicBasic {
          slot_type: IRSlotType::DYNAMIC,
          name: arg.unwrap(),
          _fn: block,
          _loop: None,
        },
      },
    )));
  } else if let Some(v_else) = v_else {
    let v_else_dir = resolve_directive(v_else, context)?;
    if let Some(last_slot) = _slots.last_mut() {
      if let Either3::B(Either3::C(v_if_slot)) = last_slot {
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
        on_error(env, ErrorCodes::X_V_ELSE_NO_ADJACENT_IF, context)
      }
    }
  } else if let Some(v_for) = v_for {
    let for_parse_result = get_for_parse_result(env, v_for, context)?;
    if for_parse_result.source.is_some() {
      _slots.push(Either3::B(Either3::B(IRSlotDynamicLoop {
        slot_type: IRSlotType::LOOP,
        name: arg.unwrap(),
        _fn: block,
        _loop: for_parse_result,
      })))
    }
  }

  if !is_basic {
    let slots = context.get_named_property::<Object>("slots")?;
    slots
      .get_named_property::<Function<FnArgs<(i32, i32)>, Object>>("splice")?
      .apply(slots, FnArgs::from((0, 99)))?;
    for slot in _slots {
      slots
        .get_named_property::<Function<IRSlots, Object>>("push")?
        .apply(slots, slot)?;
    }
  }

  Ok(Box::new(move || exit_block.call(())))
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
) -> Option<&'a mut HashMap<String, SlotBlockIRNode>> {
  let last_slot = slots.last();
  if slots.is_empty() || last_slot.is_some_and(|last_slot| !matches!(last_slot, Either3::A(_))) {
    slots.push(Either3::A(IRSlotsStatic {
      slot_type: IRSlotType::STATIC,
      slots: HashMap::new(),
    }));
  }
  let last_slot = slots.last_mut();
  if let Either3::A(slot) = last_slot.unwrap() {
    return Some(&mut slot.slots);
  }
  None
}

pub fn _ensure_static_slots<'a>(slots: &mut Object) -> Result<Object<'a>> {
  let len: i32 = slots.get_named_property("length")?;
  let last_index = if len > 0 { len - 1 } else { 0 };
  let last_slot = slots.get_named_property::<IRSlots>(last_index.to_string().as_str());
  if len == 0 || last_slot.is_ok_and(|last_slot| !matches!(last_slot, Either3::A(_))) {
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

fn register_slot(
  slots: &mut Vec<IRSlots>,
  name: Option<SimpleExpressionNode>,
  block: SlotBlockIRNode,
  block1: Object<'static>,
) {
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
    slots.push(Either3::B(Either3::A(IRSlotDynamicBasic {
      slot_type: IRSlotType::DYNAMIC,
      name: name.unwrap(),
      _fn: block1,
      _loop: None,
    })));
  }
}

pub fn _register_slot(
  mut slots: Object,
  name: Option<SimpleExpressionNode>,
  block: Object<'static>,
) {
  let is_static = if let Some(name) = &name {
    name.is_static
  } else {
    true
  };
  if is_static {
    let mut slots = _ensure_static_slots(&mut slots).unwrap();
    slots
      .set(
        if let Some(name) = &name {
          name.content.clone()
        } else {
          String::from("default")
        },
        block,
      )
      .unwrap();
  } else {
    let len = slots.get_named_property::<i32>("length").unwrap();
    let len = if len > 0 { len - 1 } else { 0 };
    slots
      .set(
        len.to_string().as_str(),
        IRSlotDynamicBasic {
          slot_type: IRSlotType::DYNAMIC,
          name: name.unwrap(),
          _fn: block,
          _loop: None,
        },
      )
      .unwrap();
  }
}

fn has_static_slot(slots: &Vec<IRSlots>, name: &str) -> bool {
  slots.iter().any(|slot| match slot {
    Either3::A(static_slot) => {
      matches!(static_slot.slot_type, IRSlotType::STATIC) && static_slot.slots.get(name).is_some()
    }
    _ => false,
  })
}

fn create_slot_block(
  env: Env,
  props: Option<SimpleExpressionNode>,
  node: Object<'static>,
  context: Object<'static>,
  exclude_slots: bool,
) -> Result<(Object<'static>, Box<dyn FnOnce() -> Result<()>>)> {
  let mut block = context
    .get_named_property::<Function<Object, Object>>("createBlock")?
    .call(node)?;
  block.set("props", props)?;
  let (block, exit_block) = enter_block(env, context, block, false, exclude_slots)?;
  Ok((block, exit_block))
}

fn _create_slot_block<'a>(
  props: Option<SimpleExpressionNode>,
  node: Object<'static>,
  context: Object<'static>,
  exclude_slots: bool,
) -> Result<(Object<'a>, Function<'static, (), ()>, String)> {
  let block = SlotBlockIRNode {
    _type: IRNodeTypes::BLOCK,
    node,
    dynamic: IRDynamicInfo::new(),
    effect: Vec::new(),
    operation: Vec::new(),
    returns: Vec::new(),
    temp_id: 0,
    props,
  };
  let exit_key = context.get_named_property::<i32>("exitKey")?;
  let (block, exit_block) = context
    .get_named_property::<Function<FnArgs<(SlotBlockIRNode, bool,bool)>, (Object, Function<(), ()>)>>(
      "enterBlock",
    )?
    .apply(context, FnArgs::from((block, false, exclude_slots)))?;
  Ok((block, exit_block, exit_key.to_string()))
}
