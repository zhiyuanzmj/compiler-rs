use napi::bindgen_prelude::Either3;
use oxc_ast::ast::{JSXAttribute, JSXAttributeName};
use oxc_span::SPAN;

use crate::{
  ir::index::{DirectiveNode, SimpleExpressionNode},
  transform::TransformContext,
};

pub fn resolve_directive<'a>(
  node: &'a mut JSXAttribute<'a>,
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
    let splited = &mut cloned.split("$").collect::<Vec<_>>();
    if splited.len() > 1 {
      is_static = false;
      arg_string = splited[1].replace("_", ".");
      if !splited[2].is_empty() {
        modifiers = splited[2][1..]
          .split("_")
          .map(|s| s.to_string())
          .collect::<Vec<_>>();
      }
    } else {
      let mut splited = cloned.split("_").map(|i| i.to_string()).collect::<Vec<_>>();
      arg_string = splited.remove(0);
      modifiers = splited;
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
        loc: SPAN,
      })
    } else {
      None
    }
  } else if let JSXAttributeName::Identifier(_) = &node.name {
    Some(SimpleExpressionNode {
      content: name_string,
      is_static: true,
      ast: None,
      loc: SPAN,
    })
  } else {
    None
  };

  let exp = if let Some(exp) = &mut node.value {
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
      loc: SPAN,
    })
    .collect();
  DirectiveNode {
    name: dir_name,
    exp,
    arg,
    loc: SPAN,
    modifiers: modifiers,
  }
}
