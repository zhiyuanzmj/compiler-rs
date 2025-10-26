use std::{
  cell::RefCell,
  collections::{HashMap, HashSet},
  rc::Rc,
};

use napi::{
  Either, Env, JsValue, Result, ValueType,
  bindgen_prelude::{FnArgs, Function, JsObjectValue, Object},
};
use napi_derive::napi;

use crate::utils::{
  check::{is_for_statement, is_function_type, is_identifier, is_referenced_identifier},
  extract::extract_identifiers,
  utils::TS_NODE_TYPES,
};

type SyncHandler<'a> = Box<
  dyn FnMut(
      Object<'static>,
      Option<Object<'static>>,
      Option<String>,
      Option<u32>,
    ) -> Result<Option<Either<bool, Object<'static>>>>
    + 'a,
>;

pub struct SyncWalker<'a> {
  should_skip: bool,
  should_remove: bool,
  replacement: Option<Object<'static>>,
  enter: Option<SyncHandler<'a>>,
  leave: Option<SyncHandler<'a>>,
}

impl<'a> SyncWalker<'a> {
  pub fn new(enter: Option<SyncHandler<'a>>, leave: Option<SyncHandler<'a>>) -> Self {
    Self {
      should_skip: false,
      should_remove: false,
      replacement: None,
      enter,
      leave,
    }
  }

  pub fn replace(
    &mut self,
    parent: Option<Object>,
    prop: Option<String>,
    index: Option<u32>,
    node: Object,
  ) {
    if let Some(mut parent) = parent
      && let Some(prop) = prop
    {
      if let Some(index) = index {
        parent
          .get_named_property::<Object>(&prop)
          .unwrap()
          .set(&index.to_string(), node)
          .unwrap();
      } else {
        parent.set(&prop, node).unwrap();
      }
    }
  }

  pub fn remove(&mut self, parent: Option<Object>, prop: Option<String>, index: Option<u32>) {
    if let Some(mut parent) = parent
      && let Some(prop) = prop
    {
      if let Some(index) = index {
        let arr = parent.get_named_property::<Object>(&prop).unwrap();
        arr
          .get_named_property::<Function<FnArgs<(u32, u32)>, Object>>("splice")
          .unwrap()
          .apply(arr, (index, 1).into())
          .unwrap();
      } else {
        parent.delete_named_property(&prop).unwrap();
      }
    }
  }

  pub fn visit(
    &mut self,
    mut node: Object<'static>,
    parent: Option<Object<'static>>,
    prop: Option<String>,
    index: Option<u32>,
  ) -> Result<Option<Object<'static>>> {
    if let Some(enter) = &mut self.enter {
      let _should_skip = self.should_skip.clone();
      let _should_remove = self.should_remove;
      let _replacement = self.replacement;
      self.should_skip = false;
      self.should_remove = false;
      self.replacement = None;

      if let Some(result) = enter(node, parent, prop.clone(), index)? {
        match result {
          Either::A(b) => {
            if b {
              self.should_skip = true
            } else {
              self.should_remove = true
            }
          }
          Either::B(node) => self.replacement = Some(node),
        }
      };

      if let Some(replacement) = self.replacement {
        node = replacement;
        self.replace(parent, prop.clone(), index, node);
      }

      if self.should_remove {
        self.remove(parent, prop.clone(), index);
      }

      let skiped = self.should_skip;
      let removed = self.should_remove;

      self.should_skip = _should_skip;
      self.should_remove = _should_remove;
      self.replacement = _replacement;

      if skiped {
        return Ok(Some(node));
      }
      if removed {
        return Ok(None);
      }
    }

    let keys = node.get_property_names()?;
    for i in 0..keys.get_array_length()? {
      let key = keys.get_element::<String>(i)?;
      if let Ok(value) = node.get_named_property::<Object>(&key) {
        if value.is_array()? {
          let mut _i = 0;
          while _i < value.get_array_length()? {
            if let Ok(item) = value.get_named_property::<Object>(&_i.to_string())
              && is_node(item)
            {
              if self
                .visit(item, Some(node), Some(key.clone()), Some(_i))?
                .is_none()
              {
                continue;
              };
            }
            _i += 1;
          }
        } else if is_node(value) {
          self.visit(value, Some(node), Some(key), None)?;
        }
      };
    }

    if let Some(leave) = &mut self.leave {
      let _replacement = self.replacement;
      let _should_remove = self.should_remove;
      self.replacement = None;
      self.should_remove = false;

      if let Some(result) = leave(node, parent, prop.clone(), index)? {
        match result {
          Either::A(b) => {
            if b {
              self.should_skip = true
            } else {
              self.should_remove = true
            }
          }
          Either::B(node) => self.replacement = Some(node),
        }
      }

      if let Some(replacement) = self.replacement {
        node = replacement;
        self.replace(parent, prop.clone(), index, node);
      }

      if self.should_remove {
        self.remove(parent, prop.clone(), index);
      }

      let removed = self.should_remove;

      self.replacement = _replacement;
      self.should_remove = _should_remove;

      if removed {
        return Ok(None);
      }
    }

    return Ok(Some(node));
  }
}

fn is_node(value: Object<'static>) -> bool {
  if let Ok(ValueType::Null) = value.to_unknown().get_type() {
    return false;
  }
  value.get_named_property::<String>("type").is_ok()
}

#[napi(js_name = "SyncHandler<T = object>")]
pub type _SyncHandler<T = Object<'static>> = Function<
  'static,
  FnArgs<(T, Option<T>, Option<String>, Option<u32>)>,
  Option<Either<bool, Object<'static>>>,
>;
#[napi(object)]
pub struct WalkOptions {
  pub enter: Option<_SyncHandler<Object<'static>>>,
  pub leave: Option<_SyncHandler<Object<'static>>>,
}
#[napi]
pub fn walk(ast: Object<'static>, options: WalkOptions) -> Result<Option<Object<'static>>> {
  let mut i = SyncWalker::new(
    if let Some(enter) = options.enter {
      Some(Box::new(move |node, parent, prop, index| {
        enter.call((node, parent, prop, index).into())
      }))
    } else {
      None
    },
    if let Some(leave) = options.leave {
      Some(Box::new(move |node, parent, prop, index| {
        leave.call((node, parent, prop, index).into())
      }))
    } else {
      None
    },
  );
  return i.visit(ast, None, None, None);
}

#[napi]
/**
 * Modified from https://github.com/vuejs/core/blob/main/packages/compiler-core/src/babelUtils.ts
 * To support browser environments and JSX.
 *
 * https://github.com/vuejs/core/blob/main/LICENSE
 *
 * Return value indicates whether the AST walked can be a constant
 */
pub fn walk_identifiers(
  env: Env,
  root: Object<'static>,
  on_identifier: Function<
    FnArgs<(
      Object<'static>,
      Option<Object<'static>>,
      Vec<Object<'static>>,
      bool,
      bool,
    )>,
  >,
  include_all: Option<bool>,
  parent_stack: Option<Vec<Object<'static>>>,
  known_ids: Option<HashMap<String, u32>>,
) -> Result<()> {
  _walk_identifiers(
    env,
    root,
    |node, parent, parent_stack, is_refed, is_local| {
      on_identifier.call((node, parent, parent_stack, is_refed, is_local).into());
      Ok(())
    },
    include_all.unwrap_or(false),
    parent_stack,
    known_ids,
  )
}
pub fn _walk_identifiers(
  env: Env,
  root: Object<'static>,
  mut on_identifier: impl FnMut(
    Object<'static>,
    Option<Object<'static>>,
    Vec<Object<'static>>,
    bool,
    bool,
  ) -> Result<()>,
  include_all: bool,
  parent_stack: Option<Vec<Object<'static>>>,
  known_ids: Option<HashMap<String, u32>>,
) -> Result<()> {
  let parent_stack = if let Some(parent_stack) = parent_stack {
    parent_stack
  } else {
    Vec::new()
  };
  let known_ids = if let Some(known_ids) = known_ids {
    known_ids
  } else {
    HashMap::new()
  };
  let parent_stack = Rc::new(RefCell::new(parent_stack));
  let parent_stack1 = Rc::clone(&parent_stack);
  let known_ids = Rc::new(RefCell::new(known_ids));
  let known_ids1 = Rc::clone(&known_ids);
  SyncWalker::new(
    Some(Box::new(move |mut node, parent, _, _| {
      let mut parent_stack = parent_stack.borrow_mut();
      let mut known_ids = known_ids.borrow_mut();
      let mut parent_type = String::new();
      if let Some(parent) = parent {
        parent_stack.push(parent);
        if let Ok(_type) = parent.get_named_property::<String>("type") {
          parent_type = _type.clone();
          if _type.starts_with("TS") && !TS_NODE_TYPES.contains(&_type.as_str()) {
            return Ok(Some(Either::A(true)));
          }
        }
      };

      let node_type = node.get_named_property::<String>("type")?;
      if is_identifier(node) {
        let is_local = known_ids.contains_key(&node.get_named_property::<String>("name")?);
        let is_refed = is_referenced_identifier(env, node, parent, parent_stack.clone())?;
        if include_all || (is_refed && !is_local) {
          on_identifier(node, parent, parent_stack.clone(), is_refed, is_local)?;
        }
      } else if node_type == "Property" && parent_type == "ObjectPattern" {
        // mark property in destructure pattern
        node.set_named_property("inPattern", true)?;
      } else if is_function_type(node) {
        if let Ok(scope_ids) = node.get_named_property::<Vec<String>>("scopeIds") {
          scope_ids.into_iter().for_each(|id| {
            mark_known_ids(id, &mut known_ids);
          });
        } else {
          // walk function expressions and add its arguments to known identifiers
          // so that we don't prefix them
          let known_ids = &mut known_ids;
          walk_function_params(node, move |id| {
            mark_scope_identifier(node, id, known_ids).unwrap();
          })?
        }
      } else if node_type == "BlockStatement" {
        if let Ok(scope_ids) = node.get_named_property::<Vec<String>>("scopeIds") {
          scope_ids.into_iter().for_each(|id| {
            mark_known_ids(id, &mut known_ids);
          });
        } else {
          // #3445 record block-level local variables
          let known_ids = &mut known_ids;
          walk_block_declarations(node, move |id| {
            mark_scope_identifier(node, id, known_ids).unwrap();
          })?;
        }
      } else if node_type == "CatchClause"
        && let Ok(param) = node.get_named_property::<Object>("param")
      {
        let known_ids = &mut known_ids;
        for id in extract_identifiers(param, vec![])? {
          mark_scope_identifier(node, id, known_ids)?;
        }
      } else if is_for_statement(node) {
        let known_ids = &mut known_ids;
        walk_for_statement(node, false, &mut move |id| {
          mark_scope_identifier(node, id, known_ids).unwrap();
        })?;
      }
      Ok(None)
    })),
    Some(Box::new(move |node, parent, _, _| {
      let mut known_ids = known_ids1.borrow_mut();
      if parent.is_some() {
        parent_stack1.borrow_mut().pop();
      }
      if !env.strict_equals(node, root)?
        && let Ok(scope_ids) = node.get_named_property::<HashSet<String>>("scopeIds")
      {
        for id in scope_ids {
          let size = known_ids[&id];
          known_ids.insert(id.clone(), size - 1);
          if known_ids[&id] == 0 {
            known_ids.remove(&id);
          }
        }
      }
      Ok(None)
    })),
  )
  .visit(root, None, None, None)?;
  Ok(())
}

pub fn walk_function_params<'a>(node: Object, mut on_ident: impl FnMut(Object) + 'a) -> Result<()> {
  for p in node.get_named_property::<Vec<Object>>("params")? {
    for id in extract_identifiers(p, Vec::new())? {
      on_ident(id)
    }
  }
  Ok(())
}

pub fn walk_block_declarations<'a>(
  node: Object,
  mut on_ident: impl FnMut(Object) + 'a,
) -> Result<()> {
  for stmt in node.get_named_property::<Vec<Object>>("body")? {
    let stmt_type = stmt.get_named_property::<String>("type")?;
    if stmt_type == "VariableDeclaration" {
      if stmt.get_named_property::<bool>("declare")? {
        continue;
      }
      for decl in stmt.get_named_property::<Vec<Object>>("declarations")? {
        for id in extract_identifiers(decl.get_named_property::<Object>("id")?, Vec::new())? {
          on_ident(id)
        }
      }
    } else if stmt_type == "FunctionDeclaration" || stmt_type == "ClassDeclaration" {
      if stmt.get_named_property::<bool>("declare")?
        || stmt.get_named_property::<Object>("id").is_err()
      {
        continue;
      }
      on_ident(stmt.get_named_property::<Object>("id")?);
    } else if is_for_statement(stmt) {
      walk_for_statement(stmt, true, &mut on_ident)?;
    }
  }
  Ok(())
}

pub fn walk_for_statement(
  stmt: Object,
  is_var: bool,
  on_ident: &mut impl FnMut(Object),
) -> Result<()> {
  let variable = if stmt
    .get_named_property::<String>("type")?
    .eq("ForStatement")
  {
    stmt.get_named_property::<Object>("init")?
  } else {
    stmt.get_named_property::<Object>("left")?
  };
  if variable
    .get_named_property::<String>("type")?
    .eq("VariableDeclaration")
    && if variable.get_named_property::<String>("kind")?.eq("var") {
      is_var
    } else {
      !is_var
    }
  {
    for decl in variable.get_named_property::<Vec<Object>>("declarations")? {
      for id in extract_identifiers(decl.get_named_property::<Object>("id")?, Vec::new())? {
        on_ident(id)
      }
    }
  }

  Ok(())
}

pub fn mark_known_ids(name: String, known_ids: &mut HashMap<String, u32>) {
  if let Some(ids) = known_ids.get(&name) {
    known_ids.insert(name, ids + 1);
  } else {
    known_ids.insert(name, 1);
  }
}

pub fn mark_scope_identifier(
  mut node: Object,
  child: Object,
  known_ids: &mut HashMap<String, u32>,
) -> Result<()> {
  let name = child.get_named_property::<String>("name")?;
  if let Ok(mut scope_ids) = node.get_named_property::<HashSet<String>>("scopeIds") {
    if scope_ids.contains(&name) {
      return Ok(());
    } else {
      scope_ids.insert(name.clone());
      node.set_named_property::<HashSet<String>>("scopeIds", scope_ids)?
    }
  } else {
    node.set_named_property::<HashSet<String>>("scopeIds", HashSet::from([name.clone()]))?
  }
  mark_known_ids(name, known_ids);
  Ok(())
}
