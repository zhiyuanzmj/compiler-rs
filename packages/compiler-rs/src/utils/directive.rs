use napi::bindgen_prelude::Either3;
use oxc_ast::ast::{JSXAttribute, JSXAttributeName};

use crate::{
  ir::index::{DirectiveNode, SimpleExpressionNode},
  transform::TransformContext,
};

pub fn resolve_directive<'a>(
  node: &JSXAttribute,
  context: &TransformContext<'a>,
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
    let cloned = arg_string.clone();
    let result = &mut cloned.split("$");
    if result.count() > 1 {
      is_static = false;
      result.next();
      arg_string = result.next().unwrap().replace("_", ".");
      if let Some(modifier_string) = result.next() {
        modifiers = modifier_string[1..]
          .split("_")
          .map(|s| s.to_string())
          .collect::<Vec<_>>();
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
