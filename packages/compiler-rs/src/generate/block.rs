use std::cell::RefCell;
use std::rc::Rc;

use napi::bindgen_prelude::{Either3, Either4, Object};
use napi::bindgen_prelude::{Function, JsObjectValue};
use napi::{Either, Env, Result};
use napi_derive::napi;

use crate::generate::operation::{gen_effects, gen_operations};
use crate::generate::template::gen_self;
use crate::generate::utils::FragmentSymbol::IndentEnd;
use crate::generate::utils::FragmentSymbol::IndentStart;
use crate::generate::utils::FragmentSymbol::Newline;
use crate::generate::utils::{
  CodeFragments, gen_call, gen_multi, get_delimiters_array, to_valid_asset_id,
};
use crate::ir::index::RootIRNode;
use crate::{generate::utils::CodeFragment, ir::index::BlockIRNode};

#[napi]
pub fn gen_block(
  env: Env,
  oper: BlockIRNode,
  context: Object,
  args: Vec<CodeFragment>,
  root: bool,
) -> Result<Vec<CodeFragment>> {
  let mut result = vec![Either3::C(Some("(".to_string()))];
  result.extend(args);
  result.push(Either3::C(Some(") => {".to_string())));
  result.push(Either3::A(IndentStart));
  result.extend(gen_block_content(env, oper, context, root, None)?);
  result.push(Either3::A(IndentEnd));
  result.push(Either3::A(Newline));
  result.push(Either3::C(Some("}".to_string())));
  Ok(result)
}

pub fn gen_block_content(
  env: Env,
  block: BlockIRNode,
  context: Object,
  root: bool,
  gen_effects_extra_frag: Option<Box<dyn FnOnce() -> Vec<CodeFragment>>>,
) -> Result<Vec<CodeFragment>> {
  let mut frag = vec![];
  let reset_block = context
    .get_named_property::<Function<BlockIRNode, Function<(), BlockIRNode>>>("enterBlock")?
    .apply(context, block)?;
  let BlockIRNode {
    _type,
    dynamic,
    effect,
    operation,
    returns,
    ..
  } = context.get_named_property::<BlockIRNode>("block")?;

  if root {
    let ir = context.get_named_property::<RootIRNode>("ir")?;
    for name in ir.component {
      let id = to_valid_asset_id(name.clone(), "component".to_string());
      frag.push(Either3::A(Newline));
      frag.push(Either3::C(Some(format!("const {id} = "))));
      frag.extend(gen_call(
        Either::A(
          context
            .get_named_property::<Function<String, String>>("helper")?
            .call("resolveComponent".to_string())?,
        ),
        vec![Either4::C(Some(format!("\"{name}\"")))],
      ))
    }
    for name in ir.directive {
      frag.push(Either3::A(Newline));
      frag.push(Either3::C(Some(format!(
        "const {} = ",
        to_valid_asset_id(name.clone(), "directive".to_string())
      ))));
      frag.extend(gen_call(
        Either::A(
          context
            .get_named_property::<Function<String, String>>("helper")?
            .call("resolveDirective".to_string())?,
        ),
        vec![Either4::C(Some(format!("\"{name}\"")))],
      ));
    }
  }

  for child in dynamic.children {
    frag.extend(gen_self(env, child, context)?);
  }

  frag.extend(gen_operations(env, operation, context)?);
  let mut effects_frag = gen_effects(env, effect, context)?;
  if let Some(gen_extra_frag) = gen_effects_extra_frag {
    effects_frag.extend(gen_extra_frag())
  }
  frag.extend(effects_frag);

  frag.push(Either3::A(Newline));
  frag.push(Either3::C(Some("return ".to_string())));

  let return_nodes = returns
    .iter()
    .map(|n| Either4::C(Some(format!("n{n}"))))
    .collect::<Vec<CodeFragments>>();
  let returns_code = if &return_nodes.len() > &1 {
    gen_multi(get_delimiters_array(), return_nodes)
  } else if let Either4::C(ref node) = return_nodes[0] {
    vec![Either3::C(Some(if let Some(node) = node {
      node.clone()
    } else {
      "null".to_string()
    }))]
  } else {
    vec![]
  };
  frag.extend(returns_code);

  reset_block.call(())?;
  Ok(frag)
}
