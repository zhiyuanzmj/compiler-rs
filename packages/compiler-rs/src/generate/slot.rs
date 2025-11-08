use std::collections::{HashMap, HashSet};

use napi::bindgen_prelude::{Either, Either3, Either4};
use oxc_ast_visit::Visit;

use crate::{
  generate::{
    CodegenContext,
    block::gen_block,
    expression::gen_expression,
    utils::{
      CodeFragment, FragmentSymbol, gen_call, gen_multi, get_delimiters_array_newline,
      get_delimiters_object_newline,
    },
  },
  ir::{
    component::{IRSlotDynamicBasic, IRSlotDynamicConditional, IRSlots},
    index::{BlockIRNode, IRFor},
  },
  utils::walk::WalkIdentifiers,
};

pub fn gen_raw_slots<'a>(
  mut slots: Vec<IRSlots<'a>>,
  context: &'a CodegenContext<'a>,
  context_block: &'a mut BlockIRNode<'a>,
) -> Option<Vec<CodeFragment>> {
  if slots.len() == 0 {
    return None;
  }
  if let Either4::A(_) = &slots[0] {
    // single static slot
    let static_slots = slots.remove(0);
    if let Either4::A(static_slots) = static_slots {
      Some(gen_static_slots(
        static_slots.slots,
        context,
        context_block,
        if slots.len() > 1 { Some(slots) } else { None },
      ))
    } else {
      None
    }
  } else {
    Some(gen_static_slots(
      HashMap::new(),
      context,
      context_block,
      Some(slots),
    ))
  }
}

fn gen_static_slots<'a>(
  mut slots: HashMap<String, BlockIRNode<'a>>,
  context: &'a CodegenContext<'a>,
  context_block: &'a mut BlockIRNode<'a>,
  dynamic_slots: Option<Vec<IRSlots<'a>>>,
) -> Vec<CodeFragment> {
  let mut args = vec![];
  let context_block = context_block as *mut BlockIRNode;
  for name in slots.keys().cloned().collect::<Vec<String>>() {
    let mut result = vec![Either3::C(Some(format!("\"{}\": ", name.clone())))];
    let oper = slots.remove(&name).unwrap();
    result.extend(gen_slot_block_with_props(oper, context, unsafe {
      &mut *context_block
    }));
    args.push(Either4::D(result))
  }
  if let Some(dynamic_slots) = dynamic_slots {
    let mut body = vec![Either3::C(Some("$: ".to_string()))];
    body.extend(gen_dynamic_slots(dynamic_slots, context, unsafe {
      &mut *context_block
    }));
    args.push(Either4::D(body));
  }
  gen_multi(get_delimiters_object_newline(), args)
}

fn gen_dynamic_slots<'a>(
  slots: Vec<IRSlots<'a>>,
  context: &'a CodegenContext<'a>,
  context_block: &'a mut BlockIRNode<'a>,
) -> Vec<CodeFragment> {
  let mut result = vec![];
  let context_block = context_block as *mut BlockIRNode;
  for slot in slots {
    result.push(match slot {
      Either4::A(slot) => Either4::D(gen_static_slots(
        slot.slots,
        context,
        unsafe { &mut *context_block },
        None,
      )),
      Either4::B(slot) => Either4::D(gen_dynamic_slot(
        slot,
        context,
        unsafe { &mut *context_block },
        true,
      )),
      Either4::C(slot) => Either4::D(gen_conditional_slot(
        slot,
        context,
        unsafe { &mut *context_block },
        true,
      )),
      Either4::D(slot) => Either4::C(Some(slot.slots.content)),
    })
  }
  gen_multi(get_delimiters_array_newline(), result)
}

fn gen_dynamic_slot<'a>(
  slot: IRSlotDynamicBasic<'a>,
  context: &'a CodegenContext<'a>,
  context_block: &'a mut BlockIRNode<'a>,
  with_function: bool,
) -> Vec<CodeFragment> {
  let frag = if slot._loop.is_none() {
    gen_basic_dynamic_slot(slot, context, context_block)
  } else {
    gen_loop_slot(slot, context, context_block)
  };
  if with_function {
    let mut result = vec![Either3::C(Some("() => (".to_string()))];
    result.extend(frag);
    result.push(Either3::C(Some(")".to_string())));
    result
  } else {
    frag
  }
}

fn gen_basic_dynamic_slot<'a>(
  slot: IRSlotDynamicBasic<'a>,
  context: &'a CodegenContext<'a>,
  context_block: &'a mut BlockIRNode<'a>,
) -> Vec<CodeFragment> {
  let mut name = vec![Either3::C(Some("name: ".to_string()))];
  name.extend(gen_expression(slot.name, context, None, None));
  let mut _fn = vec![Either3::C(Some("fn: ".to_string()))];
  _fn.extend(gen_slot_block_with_props(slot._fn, context, context_block));
  gen_multi(
    get_delimiters_object_newline(),
    vec![Either4::D(name), Either4::D(_fn)],
  )
}

fn gen_loop_slot<'a>(
  slot: IRSlotDynamicBasic<'a>,
  context: &'a CodegenContext<'a>,
  context_block: &'a mut BlockIRNode<'a>,
) -> Vec<CodeFragment> {
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
  name_expr.extend(context.with_id(|| gen_expression(name, context, None, None), &id_map));
  let mut fn_expr = vec![Either3::C(Some("fn: ".to_string()))];
  fn_expr.extend(context.with_id(
    || gen_slot_block_with_props(_fn, context, context_block),
    &id_map,
  ));
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
    Either::A(context.helper("createForSlots")),
    vec![
      Either4::D(gen_expression(source.unwrap(), context, None, None)),
      Either4::D(body),
    ],
  );
  result
}

fn gen_conditional_slot<'a>(
  slot: IRSlotDynamicConditional<'a>,
  context: &'a CodegenContext<'a>,
  context_block: &'a mut BlockIRNode<'a>,
  with_function: bool,
) -> Vec<CodeFragment> {
  let IRSlotDynamicConditional {
    condition,
    positive,
    negative,
    ..
  } = slot;
  let mut frag: Vec<CodeFragment> = vec![];
  frag.extend(gen_expression(condition, context, None, None));
  frag.extend(vec![
    Either3::A(FragmentSymbol::IndentStart),
    Either3::A(FragmentSymbol::Newline),
    Either3::C(Some("? ".to_string())),
  ]);
  let context_block = context_block as *mut BlockIRNode;
  frag.extend(gen_dynamic_slot(
    positive,
    context,
    unsafe { &mut *context_block },
    false,
  ));
  frag.push(Either3::A(FragmentSymbol::Newline));
  frag.push(Either3::C(Some(": ".to_string())));
  frag.extend(if let Some(negative) = negative {
    match *negative {
      Either::A(negative) => {
        gen_dynamic_slot(negative, context, unsafe { &mut *context_block }, false)
      }
      Either::B(negative) => {
        gen_conditional_slot(negative, context, unsafe { &mut *context_block }, false)
      }
    }
  } else {
    vec![Either3::C(Some("void 0".to_string()))]
  });
  frag.push(Either3::A(FragmentSymbol::IndentEnd));

  if with_function {
    let mut result = vec![Either3::C(Some("() => (".to_string()))];
    result.extend(frag);
    result.push(Either3::C(Some(")".to_string())));
    result
  } else {
    frag
  }
}

fn gen_slot_block_with_props<'a>(
  oper: BlockIRNode<'a>,
  context: &'a CodegenContext<'a>,
  context_block: &'a mut BlockIRNode<'a>,
) -> Vec<CodeFragment> {
  let mut is_destructure_assignment = false;
  let mut props_name = String::new();
  let mut exit_scope = None;
  let mut ids_of_props = HashSet::new();

  if let Some(props) = &oper.props {
    let raw_props = props.content.clone();
    is_destructure_assignment = props.ast.is_some();
    if is_destructure_assignment {
      let scope = context.enter_scope();
      props_name = format!("_slotProps{}", scope.0);
      if let Some(ast) = &props.ast {
        WalkIdentifiers::new(
          Box::new(|id, _, _, _, _| {
            ids_of_props.insert(id.name.to_string());
          }),
          false,
        )
        .visit_expression(ast);
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
  let block_fn = context.with_id(
    move || {
      gen_block(
        oper,
        context,
        context_block,
        vec![Either3::C(Some(props_name.clone()))],
        false,
      )
    },
    &id_map,
  );
  if let Some(exit_scope) = exit_scope {
    exit_scope();
  };
  block_fn
}
