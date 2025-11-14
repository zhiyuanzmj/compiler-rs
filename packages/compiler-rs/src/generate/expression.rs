use std::collections::HashSet;

use napi::bindgen_prelude::Either3;
use oxc_ast::{AstKind, ast::Expression};
use oxc_ast_visit::Visit;
use oxc_span::{GetSpan, Span};

use crate::{
  generate::{
    CodegenContext,
    utils::{CodeFragment, NewlineType},
  },
  ir::index::{SimpleExpressionNode, SourceLocation},
  utils::walk::WalkIdentifiers,
};

pub fn gen_expression(
  node: SimpleExpressionNode,
  context: &CodegenContext,
  assignment: Option<String>,
  need_wrap: Option<bool>,
) -> Vec<CodeFragment> {
  let content = node.content.clone();
  let loc = node.loc;
  let need_wrap = need_wrap.unwrap_or(false);

  if node.is_static {
    return vec![Either3::B((
      format!("\"{content}\""),
      NewlineType::None,
      loc,
      None,
    ))];
  }

  if content.is_empty() || node.is_constant_expression() {
    return vec![
      Either3::B((content, NewlineType::None, loc, None)),
      Either3::B((
        if let Some(assignment) = assignment {
          format!(" = {assignment}")
        } else {
          "".to_string()
        },
        NewlineType::None,
        None,
        None,
      )),
    ];
  }

  let Some(ast) = &node.ast else {
    return gen_identifier(content, context, loc, assignment.as_ref(), false);
  };

  let mut ids: Vec<Span> = vec![];
  let mut shorthands: HashSet<u32> = HashSet::new();
  if let Expression::Identifier(ast) = ast {
    ids.push(ast.span)
  } else {
    WalkIdentifiers::new(
      Box::new(|id, parent, _, _, _| {
        ids.push(id.span());
        if let Some(AstKind::AssignmentTargetPropertyIdentifier(_)) = parent {
          shorthands.insert(id.span().start);
        } else if let Some(AstKind::ObjectProperty(parent)) = parent
          && parent.shorthand
        {
          shorthands.insert(id.span().start);
        }
      }),
      false,
    )
    .visit_expression(ast);
  }

  let len = ids.len();
  if len > 0 {
    let mut frag = vec![];
    if need_wrap {
      frag.push(Either3::C(Some("() => (".to_string())));
    }
    let offset = ast.span().start as usize;
    let mut i = 0;
    for id in ids.iter() {
      let start = id.start as usize - offset;
      let end = id.end as usize - offset;
      let prev = if i > 0 { ids.get(i - 1) } else { None };

      let leading_text = content[if let Some(prev) = prev {
        prev.end as usize - offset
      } else {
        0
      }..start]
        .to_string();
      if !leading_text.is_empty() {
        frag.push(Either3::B((
          leading_text,
          NewlineType::Unknown,
          None,
          Some(" ".to_string()),
        )))
      }

      let source = content[start..end].to_string();
      let shorthand = shorthands.contains(&id.start);

      frag.extend(gen_identifier(
        source, context, None,
        // {
        //   start: advancePositionWithClone(node.loc?.start, source, start),
        //   end: advancePositionWithClone(node.loc?.start, source, end),
        // },
        None, shorthand,
      ));

      if i == len - 1 && end < content.len() {
        frag.push(Either3::B((
          content[end..].to_string(),
          NewlineType::Unknown,
          None,
          None,
        )))
      }
      i += 1;
    }

    if let Some(assignment) = assignment
      && !assignment.is_empty()
    {
      frag.push(Either3::C(Some(format!(" = {assignment}"))))
    }

    if need_wrap {
      frag.push(Either3::C(Some(")".to_string())))
    }
    frag
  } else {
    vec![Either3::B((content, NewlineType::Unknown, loc, None))]
  }
}

pub fn gen_identifier(
  mut name: String,
  context: &CodegenContext,
  loc: Option<SourceLocation>,
  assignment: Option<&String>,
  shorthand: bool,
) -> Vec<CodeFragment> {
  if let Some(id_map) = context.identifiers.borrow().get(&name)
    && id_map.len() > 0
  {
    if let Some(replacement) = id_map.get(0) {
      if shorthand {
        return vec![Either3::B((
          format!("{name}: {replacement}"),
          NewlineType::None,
          loc,
          None,
        ))];
      } else {
        return vec![Either3::B((
          replacement.to_string(),
          NewlineType::None,
          loc,
          None,
        ))];
      }
    }
  }

  let mut prefix = String::new();
  if shorthand {
    // property shorthand like { foo }, we need to add the key since
    // we rewrite the value
    prefix = format!("{name}: ");
  }

  if let Some(assignment) = assignment {
    name = format!("{name} = {assignment}");
  }

  vec![
    Either3::B((prefix, NewlineType::None, None, None)),
    Either3::B((name.clone(), NewlineType::None, loc, Some(name))),
  ]
}
