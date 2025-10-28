pub mod block;
pub mod directive;
pub mod dom;
pub mod event;
pub mod expression;
pub mod html;
pub mod operation;
pub mod template;
pub mod template_ref;
pub mod utils;
pub mod v_if;
pub mod v_model;
pub mod v_show;

use std::{
  collections::{HashMap, HashSet},
  mem,
};

use napi::{
  Result,
  bindgen_prelude::{Function, Object},
};
use napi_derive::napi;

use crate::ir::index::{BlockIRNode, RootIRNode};

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
  pub options: CodegenOptions,
  pub helpers: HashSet<String>,
  pub delegates: HashSet<String>,
  pub identifiers: HashMap<String, Vec<String>>,
  pub ir: RootIRNode,
  pub block: BlockIRNode,
  pub scope_level: i32,
}

impl CodegenContext {
  pub fn new(mut ir: RootIRNode, options: CodegenOptions) -> CodegenContext {
    let block = mem::take(&mut ir.block);
    CodegenContext {
      options,
      helpers: HashSet::new(),
      delegates: HashSet::new(),
      identifiers: HashMap::new(),
      block,
      scope_level: 0,
      ir,
    }
  }

  pub fn helper(&mut self, name: String) -> String {
    self.helpers.insert(name.clone());
    format!("_{name}")
  }

  pub fn with_id(
    &mut self,
    _fn: Function<(), Object<'static>>,
    mut map: HashMap<String, String>,
  ) -> Result<Object<'static>> {
    let ids = self.identifiers.keys().cloned().collect::<Vec<_>>();
    for ref id in ids {
      if self.identifiers.get(id).is_none() {
        self.identifiers.insert(id.to_string(), vec![]);
      }
      self.identifiers.get_mut(id).unwrap().insert(
        0,
        if let Some(id) = map.get_mut(id) {
          id.clone()
        } else {
          id.clone()
        },
      );
    }

    let ret = _fn.call(());

    let ids = self.identifiers.keys().cloned().collect::<Vec<_>>();
    let len = ids.len();
    for ref id in ids.clone() {
      if let Some(ids) = self.identifiers.get_mut(id) {
        ids.splice(0..len, vec![]);
      }
    }

    return ret;
  }
}
