use std::{rc::Rc, sync::LazyLock};

use napi::bindgen_prelude::Either3;
use oxc_ast::ast::{JSXAttribute, JSXAttributeName};
use regex::Regex;

use crate::{
  ir::index::{DirectiveNode, SimpleExpressionNode},
  transform::TransformContext,
};

static NAMESPACE_REGEX: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"^(?:\$([\w-]+)\$)?([\w-]+)?").unwrap());
pub fn resolve_directive<'a>(
  node: &JSXAttribute,
  context: &Rc<TransformContext<'a>>,
) -> DirectiveNode<'a> {
  let mut arg_string = String::new();
  let mut name_string = match &node.name {
    JSXAttributeName::Identifier(name) => name.name.to_string(),
    JSXAttributeName::NamespacedName(name) => {
      arg_string = name.name.name.to_string();
      name.namespace.name.to_string()
    }
  };
  let is_directive = name_string.starts_with("v-");
  let mut modifiers: Vec<String> = vec![];
  let mut is_static = true;

  if !matches!(node.name, JSXAttributeName::NamespacedName(_)) {
    let name_string_splited: Vec<&str> = name_string.split("_").collect();
    if name_string_splited.len() > 1 {
      modifiers = name_string_splited[1..]
        .iter()
        .map(|s| s.to_string())
        .collect();
      name_string = name_string_splited[0].to_string();
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
    if !arg_string.is_empty()
      && let JSXAttributeName::NamespacedName(_) = &node.name
    {
      Some(SimpleExpressionNode {
        content: arg_string,
        is_static,
        ast: None,
        loc: None,
      })
    } else {
      None
    }
  } else if let JSXAttributeName::Identifier(_) = &node.name {
    Some(SimpleExpressionNode {
      content: name_string,
      is_static: true,
      ast: None,
      loc: None,
    })
  } else {
    None
  };

  let exp = if let Some(exp) = &node.value {
    Some(SimpleExpressionNode::new(Either3::C(exp), context))
  } else {
    None
  };

  let modifiers = modifiers
    .into_iter()
    .map(|modifier| SimpleExpressionNode {
      content: modifier,
      is_static: false,
      ast: None,
      loc: None,
    })
    .collect();
  DirectiveNode {
    name: dir_name,
    exp,
    arg,
    loc: None,
    modifiers: modifiers,
  }
}
