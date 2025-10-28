use napi::Either;
use napi::Env;
use napi::Result;
use napi::bindgen_prelude::Either3;
use napi::bindgen_prelude::Either4;
use napi::bindgen_prelude::Either16;
use napi::bindgen_prelude::FnArgs;
use napi::bindgen_prelude::Function;
use napi::bindgen_prelude::JsObjectValue;
use napi::bindgen_prelude::Object;
use napi_derive::napi;

use crate::generate::directive::gen_builtin_directive;
use crate::generate::dom::gen_insert_node;
use crate::generate::event::gen_set_dynamic_events;
use crate::generate::event::gen_set_event;
use crate::generate::html::gen_set_html;
use crate::generate::template_ref::gen_declare_old_ref;
use crate::generate::template_ref::gen_set_template_ref;
use crate::generate::utils::CodeFragment;
use crate::generate::utils::FragmentSymbol::Newline;
use crate::generate::utils::gen_call;
use crate::ir::index::CreateComponentIRNode;
use crate::ir::index::CreateNodesIRNode;
use crate::ir::index::ForIRNode;
use crate::ir::index::GetTextChildIRNode;
use crate::ir::index::IfIRNode;
use crate::ir::index::OperationNode;
use crate::ir::index::SetDynamicPropsIRNode;
use crate::ir::index::SetNodesIRNode;
use crate::ir::index::SetPropIRNode;
use crate::ir::index::SetTextIRNode;

#[napi]
pub fn gen_operations(
  env: Env,
  opers: Vec<OperationNode>,
  context: Object,
) -> Result<Vec<CodeFragment>> {
  let mut frag = vec![];
  for operation in opers {
    frag.extend(gen_operation_with_insertion_state(env, operation, context)?);
  }
  return Ok(frag);
}

#[napi]
pub fn gen_operation_with_insertion_state(
  env: Env,
  oper: OperationNode,
  context: Object,
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

  frag.extend(gen_operation(env, oper, context)?);

  Ok(frag)
}

#[napi]
pub fn gen_operation(env: Env, oper: OperationNode, context: Object) -> Result<Vec<CodeFragment>> {
  match oper {
    Either16::A(v_if_ir_node) => context
      .get_named_property::<Function<FnArgs<(IfIRNode, Object)>, Vec<CodeFragment>>>(
        "genOperation",
      )?
      .call((v_if_ir_node, context).into()),
    Either16::B(v_for_ir_node) => context
      .get_named_property::<Function<FnArgs<(ForIRNode, Object)>, Vec<CodeFragment>>>(
        "genOperation",
      )?
      .call((v_for_ir_node, context).into()),
    Either16::C(set_text_ir_node) => context
      .get_named_property::<Function<FnArgs<(SetTextIRNode, Object)>, Vec<CodeFragment>>>(
        "genOperation",
      )?
      .call((set_text_ir_node, context).into()),
    Either16::D(set_prop_ir_node) => context
      .get_named_property::<Function<FnArgs<(SetPropIRNode, Object)>, Vec<CodeFragment>>>(
        "genOperation",
      )?
      .call((set_prop_ir_node, context).into()),
    Either16::E(set_dynamic_props_ir_node) => context
      .get_named_property::<Function<FnArgs<(SetDynamicPropsIRNode, Object)>, Vec<CodeFragment>>>(
        "genOperation",
      )?
      .call((set_dynamic_props_ir_node, context).into()),
    Either16::F(set_dynamic_events_ir_node) => {
      gen_set_dynamic_events(env, set_dynamic_events_ir_node, context)
    }
    Either16::G(set_nodes_ir_node) => context
      .get_named_property::<Function<FnArgs<(SetNodesIRNode, Object)>, Vec<CodeFragment>>>(
        "genOperation",
      )?
      .call((set_nodes_ir_node, context).into()),
    Either16::H(set_event_ir_node) => gen_set_event(env, set_event_ir_node, context),
    Either16::I(set_html_ir_node) => gen_set_html(env, set_html_ir_node, context),
    Either16::J(set_template_ref_ir_node) => {
      gen_set_template_ref(env, set_template_ref_ir_node, context)
    }
    Either16::K(create_nodes_ir_node) => context
      .get_named_property::<Function<FnArgs<(CreateNodesIRNode, Object)>, Vec<CodeFragment>>>(
        "genOperation",
      )?
      .call((create_nodes_ir_node, context).into()),
    Either16::L(insert_node_ir_node) => gen_insert_node(insert_node_ir_node, context),
    Either16::M(directive_ir_node) => gen_builtin_directive(env, directive_ir_node, context),
    Either16::N(create_component_ir_node) => context
      .get_named_property::<Function<FnArgs<(CreateComponentIRNode, Object)>, Vec<CodeFragment>>>(
        "genOperation",
      )?
      .call((create_component_ir_node, context).into()),
    Either16::O(declare_old_ref_ir_node) => Ok(gen_declare_old_ref(declare_old_ref_ir_node)),
    Either16::P(get_text_child_ir_node) => context
      .get_named_property::<Function<FnArgs<(GetTextChildIRNode, Object)>, Vec<CodeFragment>>>(
        "genOperation",
      )?
      .call((get_text_child_ir_node, context).into()),
  }
}

#[napi]
pub fn gen_insertion_state(
  parent: i32,
  anchor: Option<i32>,
  context: Object,
) -> Result<Vec<CodeFragment>> {
  let mut result = vec![Either3::A(Newline)];
  result.extend(gen_call(
    Either::A(
      context
        .get_named_property::<Function<String, String>>("helper")?
        .call("setInsertionState".to_string())?,
    ),
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
