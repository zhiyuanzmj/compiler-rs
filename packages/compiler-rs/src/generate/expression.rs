use std::collections::HashMap;

use napi::{
  Env, Result,
  bindgen_prelude::{JsObjectValue, Object},
};
use napi_derive::napi;

use crate::{
  generate::utils::{Fragment, NewlineType},
  ir::index::{SimpleExpressionNode, SourceLocation},
  utils::{
    check::is_static_property, expression::_is_constant_expression, utils::TS_NODE_TYPES,
    walk::_walk_identifiers,
  },
};

#[napi]
pub fn gen_expression(
  env: Env,
  node: SimpleExpressionNode,
  context: Object,
  assignment: Option<String>,
  need_wrap: Option<bool>,
) -> Result<Vec<Fragment>> {
  let content = node.content.clone();
  let loc = node.loc.clone();
  let need_wrap = need_wrap.unwrap_or(false);

  if node.is_static {
    return Ok(vec![(
      format!("\"{content}\""),
      NewlineType::None,
      loc,
      None,
    )]);
  }

  if content.is_empty() || _is_constant_expression(&node) {
    return Ok(vec![
      (content, NewlineType::None, loc, None),
      (
        if let Some(assignment) = assignment {
          format!(" = {assignment}")
        } else {
          "".to_string()
        },
        NewlineType::None,
        None,
        None,
      ),
    ]);
  }

  let Some(ast) = node.ast else {
    return gen_identifier(content, context, loc, assignment.as_ref(), None);
  };

  let mut ids = vec![];
  let mut parent_map = HashMap::new();
  let ids1 = &mut ids;
  let parent_map1 = &mut parent_map;
  _walk_identifiers(
    env,
    ast,
    move |id, parent, _, _, _| {
      ids1.push(id);
      if let Some(parent) = parent {
        parent_map1.insert(id.get_named_property::<u32>("start")?, parent);
      }
      Ok(())
    },
    false,
    None,
    None,
  )?;

  let mut has_member_expression = false;
  let len = ids.len();
  if len > 0 {
    let mut frag = vec![];
    if need_wrap {
      frag.push(("() => (".to_string(), NewlineType::None, None, None));
    }
    let is_ts_node = TS_NODE_TYPES.contains(&ast.get_named_property::<String>("type")?.as_str());
    let offset = ast.get_named_property::<u32>("start")? as usize;
    let mut i = 0;
    for id in ids.iter() {
      let start = id.get_named_property::<u32>("start")? as usize - offset;
      let end = id.get_named_property::<u32>("end")? as usize - offset;
      let prev = if i > 0 { ids.get(i - 1) } else { None };

      if !is_ts_node || i != 0 {
        let leading_text = content[if let Some(prev) = prev {
          prev.get_named_property::<u32>("end")? as usize - offset
        } else {
          0
        }..start]
          .to_string();
        if !leading_text.is_empty() {
          frag.push((
            leading_text,
            NewlineType::Unknown,
            None,
            Some(" ".to_string()),
          ))
        }
      }

      let source = content[start..end].to_string();
      let parent = parent_map.get(&id.get_named_property::<u32>("start")?);

      if !has_member_expression {
        has_member_expression = if let Some(parent) = parent
          && parent
            .get_named_property::<String>("type")?
            .eq("MemberExpression")
        {
          true
        } else {
          false
        };
      }

      for fragment in gen_identifier(
        source,
        context,
        None,
        // {
        //   start: advancePositionWithClone(node.loc?.start, source, start),
        //   end: advancePositionWithClone(node.loc?.start, source, end),
        // },
        if has_member_expression {
          None
        } else {
          assignment.as_ref()
        },
        parent,
      )? {
        frag.push(fragment);
      }

      if i == len - 1 && end < content.len() && !is_ts_node {
        frag.push((content[end..].to_string(), NewlineType::Unknown, None, None))
      }
      i += 1;
    }

    if let Some(assignment) = assignment
      && !assignment.is_empty()
      && has_member_expression
    {
      frag.push((format!(" = {assignment}"), NewlineType::None, None, None))
    }

    if need_wrap {
      frag.push((")".to_string(), NewlineType::None, None, None))
    }
    Ok(frag)
  } else {
    Ok(vec![(content, NewlineType::Unknown, loc, None)])
  }
}

pub fn gen_identifier(
  mut name: String,
  context: Object,
  loc: Option<SourceLocation>,
  assignment: Option<&String>,
  parent: Option<&Object>,
) -> Result<Vec<Fragment>> {
  let identifiers = context.get_named_property::<HashMap<String, Vec<String>>>("identifiers")?;
  if let Some(id_map) = identifiers.get(&name)
    && id_map.len() > 0
  {
    if let Some(replacement) = id_map.get(0) {
      if let Some(parent) = parent
        && parent.get_named_property::<String>("type")?.eq("Property")
        && parent.get_named_property::<bool>("shorthand")?
      {
        return Ok(vec![(
          format!("{name}: {replacement}"),
          NewlineType::None,
          loc,
          None,
        )]);
      } else {
        return Ok(vec![(
          replacement.to_string(),
          NewlineType::None,
          loc,
          None,
        )]);
      }
    }
  }

  let mut prefix = String::new();
  if let Some(parent) = parent
    && is_static_property(*parent)
    && parent.get_named_property::<bool>("shorthand")?
  {
    // property shorthand like { foo }, we need to add the key since
    // we rewrite the value
    prefix = format!("{name}: ");
  }

  if let Some(assignment) = assignment {
    name = format!("{name} = {assignment}");
  }

  Ok(vec![
    (prefix, NewlineType::None, None, None),
    (name.clone(), NewlineType::None, loc, Some(name)),
  ])
}
