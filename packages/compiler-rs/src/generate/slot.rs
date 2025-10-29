use napi_derive::napi;
use std::collections::{HashMap, HashSet};

use napi::{
  Either, Env, Result,
  bindgen_prelude::{Either3, Either4, Function, JsObjectValue, Object},
};

use crate::{
  generate::{
    block::gen_block,
    expression::gen_expression,
    utils::{
      CodeFragment, FragmentSymbol, gen_call, gen_multi, get_delimiters_array_newline,
      get_delimiters_object_newline,
    },
    with_id,
  },
  ir::{
    component::{IRSlotDynamicBasic, IRSlotDynamicConditional, IRSlots},
    index::{BlockIRNode, IRFor},
  },
  utils::{my_box::MyBox, walk::_walk_identifiers},
};

#[napi]
pub fn gen_raw_slots(
  env: Env,
  mut slots: Vec<IRSlots>,
  context: Object,
) -> Result<Option<Vec<CodeFragment>>> {
  if slots.len() == 0 {
    return Ok(None);
  }
  Ok(if let Either4::A(_) = &slots[0] {
    // single static slot
    let static_slots = slots.remove(0);
    if let Either4::A(static_slots) = static_slots {
      Some(gen_static_slots(
        env,
        static_slots.slots,
        context,
        if slots.len() > 1 { Some(slots) } else { None },
      )?)
    } else {
      None
    }
  } else {
    Some(gen_static_slots(env, HashMap::new(), context, Some(slots))?)
  })
}

fn gen_static_slots(
  env: Env,
  mut slots: HashMap<String, BlockIRNode>,
  context: Object,
  dynamic_slots: Option<Vec<IRSlots>>,
) -> Result<Vec<CodeFragment>> {
  let mut args = vec![];
  for name in slots.keys().cloned().collect::<Vec<String>>() {
    let mut result = vec![Either3::C(Some(format!("\"{}\": ", name.clone())))];
    let oper = slots.remove(&name).unwrap();
    result.extend(gen_slot_block_with_props(env, oper, context).unwrap());
    args.push(Either4::D(result))
  }
  if let Some(dynamic_slots) = dynamic_slots {
    let mut body = vec![Either3::C(Some("$: ".to_string()))];
    body.extend(gen_dynamic_slots(env, dynamic_slots, context)?);
    args.push(Either4::D(body));
  }
  Ok(gen_multi(get_delimiters_object_newline(), args))
}

fn gen_dynamic_slots(env: Env, slots: Vec<IRSlots>, context: Object) -> Result<Vec<CodeFragment>> {
  Ok(gen_multi(
    get_delimiters_array_newline(),
    slots
      .into_iter()
      .map(|slot| match slot {
        Either4::A(slot) => Either4::D(gen_static_slots(env, slot.slots, context, None).unwrap()),
        Either4::B(slot) => Either4::D(gen_dynamic_slot(env, slot, context, true).unwrap()),
        Either4::C(slot) => Either4::D(gen_conditional_slot(env, slot, context, true).unwrap()),
        Either4::D(slot) => Either4::C(Some(slot.slots.content)),
      })
      .collect::<Vec<_>>(),
  ))
}

fn gen_dynamic_slot(
  env: Env,
  slot: IRSlotDynamicBasic,
  context: Object,
  with_function: bool,
) -> Result<Vec<CodeFragment>> {
  let frag = if slot._loop.is_none() {
    gen_basic_dynamic_slot(env, slot, context)?
  } else {
    gen_loop_slot(env, slot, context)?
  };
  Ok(if with_function {
    let mut result = vec![Either3::C(Some("() => (".to_string()))];
    result.extend(frag);
    result.push(Either3::C(Some(")".to_string())));
    result
  } else {
    frag
  })
}

fn gen_basic_dynamic_slot(
  env: Env,
  slot: IRSlotDynamicBasic,
  context: Object,
) -> Result<Vec<CodeFragment>> {
  let mut name = vec![Either3::C(Some("name: ".to_string()))];
  name.extend(gen_expression(env, slot.name, context, None, None)?);
  let mut _fn = vec![Either3::C(Some("fn: ".to_string()))];
  _fn.extend(gen_slot_block_with_props(env, slot._fn, context)?);
  Ok(gen_multi(
    get_delimiters_object_newline(),
    vec![Either4::D(name), Either4::D(_fn)],
  ))
}

fn gen_loop_slot(env: Env, slot: IRSlotDynamicBasic, context: Object) -> Result<Vec<CodeFragment>> {
  let IRSlotDynamicBasic {
    name, _fn, _loop, ..
  } = slot;
  let IRFor {
    value,
    key,
    index,
    source,
    ..
  } = _loop.unwrap();
  let raw_value = value.and_then(|value| Some(value.content));
  let raw_key = key.and_then(|key| Some(key.content));
  let raw_index = index.and_then(|index| Some(index.content));

  let mut id_map = HashMap::new();
  if let Some(raw_value) = &raw_value {
    id_map.insert(raw_value.clone(), raw_value.clone());
  }
  if let Some(raw_key) = &raw_key {
    id_map.insert(raw_key.clone(), raw_key.clone());
  }
  if let Some(raw_index) = &raw_index {
    id_map.insert(raw_index.clone(), raw_index.clone());
  }

  let mut name_expr = vec![Either3::C(Some("name: ".to_string()))];
  name_expr.extend(with_id(
    env,
    context,
    || gen_expression(env, name, context, None, None),
    &id_map,
  )?);
  let mut fn_expr = vec![Either3::C(Some("fn: ".to_string()))];
  fn_expr.extend(with_id(
    env,
    context,
    || gen_slot_block_with_props(env, _fn, context),
    &id_map,
  )?);
  let slot_expr = gen_multi(
    get_delimiters_object_newline(),
    vec![Either4::D(name_expr), Either4::D(fn_expr)],
  );
  let mut body = gen_multi(
    (
      Either4::C(Some(String::from("("))),
      Either4::C(Some(String::from(")"))),
      Either4::C(Some(String::from(", "))),
      None,
    ),
    vec![
      Either4::C(if let Some(raw_value) = raw_value {
        Some(raw_value)
      } else if raw_key.is_some() && raw_index.is_some() {
        Some("_".to_string())
      } else {
        None
      }),
      Either4::C(if let Some(raw_key) = raw_key {
        Some(raw_key)
      } else if raw_key.is_some() && raw_key.is_some() {
        Some("__".to_string())
      } else {
        None
      }),
      Either4::C(raw_index),
    ],
  );
  body.push(Either3::C(Some(" => (".to_string())));
  body.extend(slot_expr);
  body.push(Either3::C(Some(")".to_string())));
  let result = gen_call(
    Either::A(
      context
        .get_named_property::<Function<String, String>>("helper")?
        .call("createForSlots".to_string())?,
    ),
    vec![
      Either4::D(gen_expression(env, source.unwrap(), context, None, None)?),
      Either4::D(body),
    ],
  );
  Ok(result)
}

fn gen_conditional_slot(
  env: Env,
  slot: IRSlotDynamicConditional,
  context: Object,
  with_function: bool,
) -> Result<Vec<CodeFragment>> {
  let IRSlotDynamicConditional {
    condition,
    positive,
    negative,
    ..
  } = slot;
  let mut frag: Vec<CodeFragment> = vec![];
  frag.extend(gen_expression(env, condition, context, None, None)?);
  frag.extend(vec![
    Either3::A(FragmentSymbol::IndentStart),
    Either3::A(FragmentSymbol::Newline),
    Either3::C(Some("? ".to_string())),
  ]);
  frag.extend(gen_dynamic_slot(env, positive, context, false)?);
  frag.push(Either3::A(FragmentSymbol::Newline));
  frag.push(Either3::C(Some(": ".to_string())));
  frag.extend(if let Some(MyBox(negative)) = negative {
    match *negative {
      Either::A(negative) => gen_dynamic_slot(env, negative, context, false)?,
      Either::B(negative) => gen_conditional_slot(env, negative, context, false)?,
    }
  } else {
    vec![Either3::C(Some("void 0".to_string()))]
  });
  frag.push(Either3::A(FragmentSymbol::IndentEnd));

  Ok(if with_function {
    let mut result = vec![Either3::C(Some("() => (".to_string()))];
    result.extend(frag);
    result.push(Either3::C(Some(")".to_string())));
    result
  } else {
    frag
  })
}

fn gen_slot_block_with_props(
  env: Env,
  oper: BlockIRNode,
  context: Object,
) -> Result<Vec<CodeFragment>> {
  let mut is_destructure_assignment = false;
  let mut props_name = String::new();
  let mut exit_scope = None;
  let mut ids_of_props = HashSet::new();

  if let Some(props) = &oper.props {
    let raw_props = props.content.clone();
    is_destructure_assignment = props.ast.is_some();
    if is_destructure_assignment {
      let scope = context
        .get_named_property::<Function<(), (i32, Function<(), i32>)>>("enterScope")?
        .apply(context, ())?;
      props_name = format!("_slotProps{}", scope.0);
      if let Some(ast) = props.ast {
        _walk_identifiers(
          env,
          ast,
          |id, _, _, is_reference, is_local| {
            if is_reference && !is_local {
              ids_of_props.insert(id.get_named_property::<String>("name")?);
            }
            Ok(())
          },
          true,
          None,
          None,
        )?;
      }
      exit_scope = Some(scope.1);
    } else {
      props_name = raw_props.clone();
      ids_of_props.insert(raw_props);
    }
  }

  let mut id_map = HashMap::new();

  for id in ids_of_props {
    id_map.insert(
      id.clone(),
      if is_destructure_assignment {
        format!("{}[\"{}\"]", props_name.as_str(), id)
      } else {
        String::new()
      },
    );
  }
  let block_fn = with_id(
    env,
    context,
    move || {
      gen_block(
        env,
        oper,
        context,
        vec![Either3::C(Some(props_name.clone()))],
        false,
      )
    },
    &id_map,
  )?;
  if let Some(exit_scope) = exit_scope {
    exit_scope.call(())?;
  };

  Ok(block_fn)
}
