use napi::{
  JsValue, Result, ValueType,
  bindgen_prelude::{FnArgs, Function, JsObjectValue, Object},
};
use napi_derive::napi;

type SyncHandler = Box<
  dyn Fn(&mut SyncWalker, Object<'static>, Option<Object<'static>>, Option<String>, Option<u32>),
>;

pub struct SyncWalker {
  should_skip: bool,
  should_remove: bool,
  replacement: Option<Object<'static>>,
  enter: Option<SyncHandler>,
  leave: Option<SyncHandler>,
}

#[napi]
impl SyncWalker {
  pub fn new(enter: SyncHandler, leave: SyncHandler) -> Self {
    Self {
      should_skip: false,
      should_remove: false,
      replacement: None,
      enter: Some(enter),
      leave: Some(leave),
    }
  }

  pub fn skip(&mut self) {
    self.should_skip = true;
  }

  pub fn remove(&mut self) {
    self.should_remove = true;
  }

  pub fn replace(&mut self, node: Object<'static>) {
    self.replacement = Some(node);
  }

  pub fn _replace(
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

  pub fn _remove(&mut self, parent: Option<Object>, prop: Option<String>, index: Option<u32>) {
    if let Some(mut parent) = parent
      && let Some(prop) = prop
    {
      if let Some(index) = index {
        parent
          .get_named_property::<Object>(&prop)
          .unwrap()
          .get_named_property::<Function<FnArgs<(u32, u32)>, Object>>("splice")
          .unwrap()
          .call(FnArgs::from((index, 1)))
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
    let this = self as *mut Self;
    if let Some(enter) = &self.enter {
      let _should_skip = self.should_skip.clone();
      let _should_remove = self.should_remove;
      let _replacement = self.replacement;
      self.should_skip = false;
      self.should_remove = false;
      self.replacement = None;

      enter(unsafe { &mut *this }, node, parent, prop.clone(), index);

      if let Some(replacement) = self.replacement {
        node = replacement;
        self._replace(parent, prop.clone(), index, node);
      }

      if self.should_remove {
        self._remove(parent, prop.clone(), index);
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
          for mut _i in 0..value.get_array_length()? {
            if let Ok(item) = value.get_named_property(&_i.to_string())
              && is_node(item)
            {
              if self
                .visit(item, Some(node), Some(key.clone()), Some(_i))?
                .is_none()
              {
                _i -= 1;
              };
            }
          }
        } else if is_node(value) {
          self.visit(value, Some(node), Some(key), None)?;
        }
      };
    }

    if let Some(leave) = &self.leave {
      let _replacement = self.replacement;
      let _should_remove = self.should_remove;
      self.replacement = None;
      self.should_remove = false;

      leave(unsafe { &mut *this }, node, parent, prop.clone(), index);

      if let Some(replacement) = self.replacement {
        node = replacement;
        self._replace(parent, prop.clone(), index, node);
      }

      if self.should_remove {
        self._remove(parent, prop.clone(), index);
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

pub fn walk(ast: Object<'static>, enter: SyncHandler, leave: SyncHandler) {
  let mut i = SyncWalker::new(Box::new(enter), Box::new(leave));
  i.visit(ast, None, None, None).unwrap();
}
