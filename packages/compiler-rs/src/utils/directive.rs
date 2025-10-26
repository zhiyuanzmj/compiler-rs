use std::{rc::Rc, sync::LazyLock};

use napi::{Result, bindgen_prelude::Object};
use regex::Regex;

use crate::{
  ir::index::DirectiveNode,
  transform::TransformContext,
  utils::expression::{create_simple_expression, get_value, resolve_expression},
};

static NAMESPACE_REGEX: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"^(?:\$([\w-]+)\$)?([\w-]+)?").unwrap());
pub fn resolve_directive(node: Object, context: &Rc<TransformContext>) -> Result<DirectiveNode> {
  let name = node.get::<Object>("name")?.expect("name is required!");
  let name_type = name
    .get::<String>("type")
    .ok()
    .flatten()
    .map_or(String::from(""), |a| a);
  let mut name_string = if name_type.eq("JSXNamespacedName") {
    name
      .get::<Object>("namespace")
      .ok()
      .flatten()
      .map_or(String::from(""), |a| {
        a.get::<String>("name")
          .ok()
          .flatten()
          .map_or(String::from(""), |b| b)
      })
  } else if name_type.eq("JSXIdentifier") {
    name
      .get::<String>("name")
      .ok()
      .flatten()
      .map_or(String::from(""), |a| a)
  } else {
    String::from("")
  };
  let is_directive = name_string.starts_with("v-");
  let mut modifiers: Vec<String> = vec![];
  let mut is_static = true;
  let mut arg_string = if name_type.eq("JSXNamespacedName") {
    name
      .get::<Object>("name")
      .ok()
      .flatten()
      .map_or(String::from(""), |name| {
        name
          .get::<String>("name")
          .ok()
          .flatten()
          .map_or(String::from(""), |b| b)
      })
  } else {
    String::from("")
  };
  if name_type != "JSXNamespacedName" && arg_string.is_empty() {
    let name_string_splited: Vec<&str> = name_string.split("_").collect();
    if name_string_splited.len() > 1 {
      modifiers = name_string_splited[1..]
        .iter()
        .map(|s| s.to_string())
        .collect();
      name_string = name_string_splited[0].to_owned();
    }
  } else {
    if let Some(result) = NAMESPACE_REGEX.captures(&arg_string.clone()) {
      arg_string = match result.get(1) {
        Some(m) => m.to_owned().as_str().to_string(),
        None => String::new(),
      };
      let modifier_string = match result.get(2) {
        Some(m) => m.as_str(),
        None => "",
      };
      if !arg_string.is_empty() {
        arg_string = arg_string.replace("_", ".");
        is_static = false;
        if modifier_string.starts_with("_") {
          modifiers = modifier_string[1..]
            .split("_")
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        }
      } else if !modifier_string.is_empty() {
        let splited: Vec<String> = modifier_string.split("_").map(|s| s.to_string()).collect();
        arg_string = splited[0].to_owned();
        modifiers = splited[1..]
          .iter()
          .map(|s| s.to_string())
          .collect::<Vec<String>>();
      }
    }
  }

  let dir_name = if is_directive {
    name_string[2..].to_string()
  } else {
    String::from("bind")
  };

  let arg = if is_directive {
    if !arg_string.is_empty() && name_type.eq("JSXNamespacedName") {
      Some(create_simple_expression(
        arg_string,
        Some(is_static),
        if is_static {
          name.get::<Object>("name").ok().flatten()
        } else {
          None
        },
        None,
      ))
    } else {
      None
    }
  } else {
    Some(create_simple_expression(
      name_string,
      Some(true),
      Some(name),
      None,
    ))
  };

  let exp = if let Some(exp) = get_value(node) {
    Some(resolve_expression(exp, context))
  } else {
    None
  };

  let modifiers = modifiers
    .iter()
    .map(|modifier| {
      create_simple_expression(modifier.to_owned().to_owned(), Some(false), None, None)
    })
    .collect();
  Ok(DirectiveNode {
    name: dir_name,
    exp,
    arg,
    loc: None,
    modifiers: modifiers,
  })
}
