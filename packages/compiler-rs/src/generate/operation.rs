use napi::Either;
use napi::Result;
use napi::bindgen_prelude::Either3;
use napi::bindgen_prelude::Either4;
use napi::bindgen_prelude::Either16;
use std::mem;

use crate::generate::CodegenContext;
use crate::generate::component::gen_create_component;
use crate::generate::directive::gen_builtin_directive;
use crate::generate::dom::gen_insert_node;
use crate::generate::event::gen_set_dynamic_events;
use crate::generate::event::gen_set_event;
use crate::generate::html::gen_set_html;
use crate::generate::prop::gen_dynamic_props;
use crate::generate::prop::gen_set_prop;
use crate::generate::template_ref::gen_declare_old_ref;
use crate::generate::template_ref::gen_set_template_ref;
use crate::generate::text::gen_create_nodes;
use crate::generate::text::gen_get_text_child;
use crate::generate::text::gen_set_nodes;
use crate::generate::text::gen_set_text;
use crate::generate::utils::CodeFragment;
use crate::generate::utils::FragmentSymbol;
use crate::generate::utils::FragmentSymbol::Newline;
use crate::generate::utils::gen_call;
use crate::generate::v_for::gen_for;
use crate::generate::v_if::gen_if;
use crate::ir::index::BlockIRNode;
use crate::ir::index::OperationNode;
use crate::ir::index::SetEventIRNode;

pub fn gen_operations(
  opers: Vec<OperationNode>,
  context: &CodegenContext,
  context_block: &mut BlockIRNode,
) -> Result<Vec<CodeFragment>> {
  let mut frag = vec![];
  let event_opers = opers
    .iter()
    .filter_map(|op| {
      if let Either16::H(op) = op {
        Some(op.clone())
      } else {
        None
      }
    })
    .collect::<Vec<_>>();
  for operation in opers {
    frag.extend(gen_operation_with_insertion_state(
      operation,
      context,
      context_block,
      &event_opers,
    )?);
  }
  return Ok(frag);
}

pub fn gen_operation_with_insertion_state(
  oper: OperationNode,
  context: &CodegenContext,
  context_block: &mut BlockIRNode,
  event_opers: &Vec<SetEventIRNode>,
) -> Result<Vec<CodeFragment>> {
  let mut frag = vec![];
  match &oper {
    Either16::A(if_ir_node) => {
      if let Some(parent) = if_ir_node.parent {
        frag.extend(gen_insertion_state(parent, if_ir_node.anchor, context)?)
      }
    }
    Either16::B(for_ir_node) => {
      if let Some(parent) = for_ir_node.parent {
        frag.extend(gen_insertion_state(parent, for_ir_node.anchor, context)?)
      }
    }
    Either16::N(create_component_ir_node) => {
      if let Some(parent) = create_component_ir_node.parent {
        frag.extend(gen_insertion_state(
          parent,
          create_component_ir_node.anchor,
          context,
        )?)
      }
    }
    _ => (),
  };

  frag.extend(gen_operation(oper, context, context_block, event_opers)?);

  Ok(frag)
}

pub fn gen_operation(
  oper: OperationNode,
  context: &CodegenContext,
  context_block: &mut BlockIRNode,
  event_opers: &Vec<SetEventIRNode>,
) -> Result<Vec<CodeFragment>> {
  match oper {
    Either16::A(oper) => gen_if(oper, context, context_block, false),
    Either16::B(oper) => gen_for(oper, context, context_block),
    Either16::C(oper) => gen_set_text(oper, context),
    Either16::D(oper) => gen_set_prop(oper, context),
    Either16::E(oper) => gen_dynamic_props(oper, context),
    Either16::F(oper) => gen_set_dynamic_events(oper, context),
    Either16::G(oper) => gen_set_nodes(oper, context),
    Either16::H(oper) => gen_set_event(oper, context, event_opers),
    Either16::I(oper) => gen_set_html(oper, context),
    Either16::J(oper) => gen_set_template_ref(oper, context),
    Either16::K(oper) => gen_create_nodes(oper, context),
    Either16::L(oper) => gen_insert_node(oper, context),
    Either16::M(oper) => gen_builtin_directive(oper, context),
    Either16::N(oper) => gen_create_component(oper, context, context_block),
    Either16::O(oper) => Ok(gen_declare_old_ref(oper)),
    Either16::P(oper) => gen_get_text_child(oper, context),
  }
}

pub fn gen_insertion_state(
  parent: i32,
  anchor: Option<i32>,
  context: &CodegenContext,
) -> Result<Vec<CodeFragment>> {
  let mut result = vec![Either3::A(Newline)];
  result.extend(gen_call(
    Either::A(context.helper("setInsertionState")),
    vec![
      Either4::C(Some(format!("n{}", parent))),
      Either4::C(if let Some(anchor) = anchor {
        if anchor == -1 {
          // -1 indicates prepend
          Some("0".to_string()) // runtime anchor value for prepend
        } else {
          Some(format!("n{anchor}"))
        }
      } else {
        None
      }),
    ],
  ));
  Ok(result)
}

pub fn gen_effects(
  context: &CodegenContext,
  context_block: &mut BlockIRNode,
) -> Result<Vec<CodeFragment>> {
  let mut frag: Vec<CodeFragment> = vec![];
  let mut operations_count = 0;

  let mut i = 0;
  let effects = mem::take(&mut context_block.effect);
  let effects_len = effects.len();
  for effect in effects {
    operations_count += effect.operations.len();
    let frags = gen_effect(effect.operations, context, context_block)?;
    if i > 0 {
      frag.push(Either3::A(Newline));
    }
    if let Some(last) = frag.last()
      && matches!(last, Either3::C(Some(s)) if s.eq(")"))
      && let Some(first) = frags.first()
      && matches!(first, Either3::C(Some(s)) if s.eq("("))
    {
      frag.push(Either3::C(Some(";".to_string())))
    }
    frag.extend(frags);
    i += 1;
  }

  let newline_count = frag
    .iter()
    .filter(|frag| matches!(frag, Either3::A(FragmentSymbol::Newline)))
    .collect::<Vec<_>>()
    .len();
  if newline_count > 1 || operations_count > 1 {
    frag.insert(0, Either3::A(FragmentSymbol::Newline));
    frag.insert(0, Either3::A(FragmentSymbol::IndentStart));
    frag.insert(0, Either3::C(Some("{".to_string())));
    frag.push(Either3::A(FragmentSymbol::IndentEnd));
    frag.push(Either3::A(FragmentSymbol::Newline));
    frag.push(Either3::C(Some("}".to_string())));
  }

  if effects_len > 0 {
    frag.insert(
      0,
      Either3::C(Some(format!("{}(() => ", context.helper("renderEffect")))),
    );
    frag.insert(0, Either3::A(FragmentSymbol::Newline));
    frag.push(Either3::C(Some(")".to_string())))
  }

  Ok(frag)
}

pub fn gen_effect(
  operations: Vec<OperationNode>,
  context: &CodegenContext,
  context_block: &mut BlockIRNode,
) -> Result<Vec<CodeFragment>> {
  let mut frag = vec![];
  let operations_exps = gen_operations(operations, context, context_block)?;
  let newline_count = operations_exps
    .iter()
    .filter(|frag| matches!(frag, Either3::A(FragmentSymbol::Newline)))
    .collect::<Vec<_>>()
    .len();

  if newline_count > 1 {
    frag.extend(operations_exps);
  } else {
    frag.extend(
      operations_exps
        .into_iter()
        .filter(|frag| !matches!(frag, Either3::A(FragmentSymbol::Newline)))
        .collect::<Vec<_>>(),
    )
  }

  Ok(frag)
}
