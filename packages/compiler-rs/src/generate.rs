pub mod block;
pub mod component;
pub mod directive;
pub mod dom;
pub mod event;
pub mod expression;
pub mod html;
pub mod operation;
pub mod prop;
pub mod slot;
pub mod template;
pub mod template_ref;
pub mod text;
pub mod utils;
pub mod v_for;
pub mod v_if;
pub mod v_model;
pub mod v_show;

use std::{
  cell::RefCell,
  collections::{HashMap, HashSet},
  mem,
};

use napi::{Env, Result, bindgen_prelude::Either3};
use napi_derive::napi;

use crate::{
  generate::{
    block::gen_block_content,
    template::gen_templates,
    utils::{CodeFragment, FragmentSymbol, code_fragment_to_string},
  },
  ir::index::{BlockIRNode, RootIRNode},
};

#[napi(object)]
pub struct CodegenOptions {
  /**
   * Generate source map?
   * @default false
   */
  pub source_map: Option<bool>,
  /**
   * Filename for source map generation.
   * Also used for self-recursive reference in templates
   * @default 'index.jsx'
   */
  pub filename: Option<String>,
  pub templates: Option<Vec<String>>,
}

pub struct CodegenContext {
  pub env: Env,
  pub options: CodegenOptions,
  pub helpers: RefCell<HashSet<String>>,
  pub delegates: RefCell<HashSet<String>>,
  pub identifiers: RefCell<HashMap<String, Vec<String>>>,
  pub ir: RootIRNode,
  pub block: RefCell<BlockIRNode>,
  pub scope_level: RefCell<i32>,
}

impl CodegenContext {
  pub fn new(env: Env, mut ir: RootIRNode, options: CodegenOptions) -> CodegenContext {
    let block = mem::take(&mut ir.block);
    CodegenContext {
      options,
      helpers: RefCell::new(HashSet::new()),
      delegates: RefCell::new(HashSet::new()),
      identifiers: RefCell::new(HashMap::new()),
      block: RefCell::new(block),
      env,
      scope_level: RefCell::new(0),
      ir,
    }
  }

  pub fn helper(&self, name: &str) -> String {
    self.helpers.borrow_mut().insert(name.to_string());
    format!("_{name}")
  }

  pub fn with_id(
    &self,
    _fn: impl FnOnce() -> Result<Vec<CodeFragment>>,
    id_map: &HashMap<String, String>,
  ) -> Result<Vec<CodeFragment>> {
    let ids = id_map.keys();
    for id in ids {
      let mut identifiers = self.identifiers.borrow_mut();
      if identifiers.get(id).is_none() {
        identifiers.insert(id.clone(), vec![]);
      }
      identifiers.get_mut(id).unwrap().insert(
        0,
        if let Some(value) = id_map.get(id) {
          if value.is_empty() {
            id.clone()
          } else {
            value.clone()
          }
        } else {
          id.clone()
        },
      );
    }

    let ret = _fn()?;

    for id in id_map.keys() {
      if let Some(ids) = self.identifiers.borrow_mut().get_mut(id) {
        ids.clear();
      }
    }

    Ok(ret)
  }

  pub fn enter_block(&self, block: BlockIRNode, context_block: &mut BlockIRNode) -> impl FnOnce() {
    let parent = mem::take(context_block);
    *context_block = block;
    || *context_block = parent
  }

  pub fn enter_scope(&self) -> (i32, impl FnOnce()) {
    let mut scope_level = self.scope_level.borrow_mut();
    let current = *scope_level;
    *scope_level += 1;
    (current, || *self.scope_level.borrow_mut() -= 1)
  }
}

#[napi(object)]
pub struct VaporCodegenResult {
  pub helpers: HashSet<String>,
  pub templates: Vec<String>,
  pub delegates: HashSet<String>,
  pub code: String,
}

// IR -> JS codegen
#[napi]
pub fn generate(env: Env, ir: RootIRNode, options: CodegenOptions) -> Result<VaporCodegenResult> {
  let mut frag = vec![];
  let has_template_ref = ir.has_template_ref;
  let root_template_index = ir.root_template_index;
  let templates = ir.templates.clone();
  let context = CodegenContext::new(env, ir, options);

  frag.push(Either3::A(FragmentSymbol::IndentStart));
  if has_template_ref {
    frag.push(Either3::A(FragmentSymbol::Newline));
    frag.push(Either3::C(Some(format!(
      "const _setTemplateRef = {}()",
      context.helper("createTemplateRefSetter")
    ))))
  }
  frag.extend(gen_block_content(
    None,
    &context,
    &mut context.block.borrow_mut(),
    true,
    None,
  )?);
  frag.push(Either3::A(FragmentSymbol::IndentEnd));
  frag.push(Either3::A(FragmentSymbol::Newline));

  if context.delegates.borrow().len() > 0 {
    context.helper("delegateEvents");
  }
  let templates = gen_templates(templates, root_template_index, &context)?;

  let code = code_fragment_to_string(frag, &context);
  Ok(VaporCodegenResult {
    code,
    delegates: context.delegates.take(),
    helpers: context.helpers.take(),
    templates,
  })
}
