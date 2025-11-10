use std::mem;

use napi::Either;
use napi::bindgen_prelude::{Either3, Either4};

use crate::generate::CodegenContext;
use crate::generate::operation::{gen_effects, gen_operations};
use crate::generate::template::gen_self;
use crate::generate::utils::FragmentSymbol::IndentEnd;
use crate::generate::utils::FragmentSymbol::IndentStart;
use crate::generate::utils::FragmentSymbol::Newline;
use crate::generate::utils::{
  CodeFragments, gen_call, gen_multi, get_delimiters_array, to_valid_asset_id,
};
use crate::{generate::utils::CodeFragment, ir::index::BlockIRNode};

pub fn gen_block<'a>(
  oper: BlockIRNode<'a>,
  context: &'a CodegenContext<'a>,
  context_block: &'a mut BlockIRNode<'a>,
  args: Vec<CodeFragment>,
  root: bool,
) -> Vec<CodeFragment> {
  let mut frag = Vec::with_capacity(256);
  frag.push(Either3::C(Some("(".to_string())));
  frag.extend(args);
  frag.push(Either3::C(Some(") => {".to_string())));
  frag.push(Either3::A(IndentStart));
  gen_block_content(&mut frag, Some(oper), context, context_block, root, None);
  frag.push(Either3::A(IndentEnd));
  frag.push(Either3::A(Newline));
  frag.push(Either3::C(Some("}".to_string())));
  frag
}

pub fn gen_block_content<'a>(
  frag: &mut Vec<CodeFragment>,
  block: Option<BlockIRNode<'a>>,
  context: &'a CodegenContext<'a>,
  context_block: &'a mut BlockIRNode<'a>,
  root: bool,
  gen_effects_extra_frag: Option<
    Box<dyn FnOnce(&'a mut BlockIRNode<'a>) -> Vec<CodeFragment> + 'a>,
  >,
) {
  let mut reset_block = None;
  let context_block = context_block as *mut BlockIRNode;
  if let Some(block) = block {
    reset_block = Some(context.enter_block(block, unsafe { &mut *context_block }));
  }

  if root {
    for name in &context.ir.component {
      let id = to_valid_asset_id(name.to_string(), "component".to_string());
      frag.push(Either3::A(Newline));
      frag.push(Either3::C(Some(format!("const {id} = "))));
      frag.extend(gen_call(
        Either::A(context.helper("resolveComponent")),
        vec![Either4::C(Some(format!("\"{name}\"")))],
      ))
    }
    for name in &context.ir.directive {
      frag.push(Either3::A(Newline));
      frag.push(Either3::C(Some(format!(
        "const {} = ",
        to_valid_asset_id(name.clone(), "directive".to_string())
      ))));
      frag.extend(gen_call(
        Either::A(context.helper("resolveDirective")),
        vec![Either4::C(Some(format!("\"{name}\"")))],
      ));
    }
  }

  for child in mem::take(&mut unsafe { &mut *context_block }.dynamic.children) {
    frag.extend(gen_self(child, context, unsafe { &mut *context_block }));
  }

  frag.extend(gen_operations(
    mem::take(&mut unsafe { &mut *context_block }.operation),
    context,
    unsafe { &mut *context_block },
  ));
  let return_nodes = unsafe { &mut *context_block }
    .returns
    .iter()
    .map(|n| Either4::C(Some(format!("n{n}"))))
    .collect::<Vec<CodeFragments>>();
  let mut effects_frag = gen_effects(context, unsafe { &mut *context_block });
  if let Some(gen_extra_frag) = gen_effects_extra_frag {
    effects_frag.extend(gen_extra_frag(unsafe { &mut *context_block }))
  }
  frag.extend(effects_frag);

  frag.push(Either3::A(Newline));
  frag.push(Either3::C(Some("return ".to_string())));

  let returns_code = if &return_nodes.len() > &1 {
    gen_multi(get_delimiters_array(), return_nodes)
  } else {
    vec![Either3::C(Some(
      if let Some(node) = return_nodes.get(0)
        && let Either4::C(Some(node)) = node
      {
        node.clone()
      } else {
        "null".to_string()
      },
    ))]
  };
  frag.extend(returns_code);

  if let Some(reset_block) = reset_block {
    reset_block();
  }
}
