use std::collections::HashMap;

use napi::{
  Either, Env, Result,
  bindgen_prelude::{Either3, Either4, Either16, JsObjectValue, Object},
};

use crate::{
  generate::{
    CodegenContext,
    block::gen_block_content,
    expression::gen_expression,
    operation::gen_operation,
    utils::{CodeFragment, FragmentSymbol, gen_call, gen_multi},
  },
  ir::index::{BlockIRNode, ForIRNode, IREffect, SimpleExpressionNode},
  utils::{
    check::is_string_literal,
    expression::is_globally_allowed,
    walk::{_walk_identifiers, SyncWalker},
  },
};

/**
 * Flags to optimize vapor `createFor` runtime behavior, shared between the
 * compiler and the runtime
 */
pub enum VaporVForFlags {
  /**
   * v-for is the only child of a parent container, so it can take the fast
   * path with textContent = '' when the whole list is emptied
   */
  FastRemove = 1,
  /**
   * v-for used on component - we can skip creating child scopes for each block
   * because the component itself already has a scope.
   */
  IsComponent = 1 << 1,
  /**
   * v-for inside v-ince
   */
  Once = 1 << 2,
}

pub fn gen_for(
  oper: ForIRNode,
  context: &CodegenContext,
  context_block: &mut BlockIRNode,
) -> Result<Vec<CodeFragment>> {
  let ForIRNode {
    source,
    value,
    key,
    index,
    _type,
    id,
    key_prop,
    mut render,
    once,
    component,
    only_child,
    ..
  } = oper;

  let mut raw_value = String::new();
  let raw_key = key.and_then(|key| Some(key.content));
  let raw_index = index.and_then(|index| Some(index.content));

  let mut source_expr = vec![Either3::C(Some("() => (".to_string()))];
  source_expr.extend(gen_expression(source, context, None, None)?);
  source_expr.push(Either3::C(Some(")".to_string())));
  let id_to_path_map = {
    // construct a id -> accessor path map.
    // e.g. `{ x: { y: [z] }}` -> `Map{ 'z' => '.x.y[0]' }`
    let mut map = HashMap::new();
    if let Some(value) = value {
      raw_value = value.content;
      if let Some(ast) = value.ast
        && !ast.get_named_property::<String>("type")?.eq("Identifier")
      {
        _walk_identifiers(
          context.env,
          ast,
          |id, _, parent_stack, is_reference, is_local| {
            if is_reference && !is_local {
              let mut path = String::new();
              let mut helper = None;
              let mut helper_args = None;
              let mut i = 0;
              for parent in &parent_stack {
                let child = parent_stack.get(i + 1).unwrap_or(&id);
                i += 1;

                let parent_type = parent.get_named_property::<String>("type")?;
                let child_type = child.get_named_property::<String>("type")?;
                if parent_type.eq("Property")
                  && context
                    .env
                    .strict_equals(parent.get_named_property::<Object>("value")?, *child)?
                  && let Ok(key) = parent.get_named_property::<Object>("key")
                {
                  if is_string_literal(Some(key)) {
                    path += &format!("[\"{}\"]", key.get_named_property::<String>("value")?);
                  } else {
                    // non-computed, can only be identifier
                    path += &format!(".{}", key.get_named_property::<String>("name")?);
                  }
                } else if parent_type.eq("ArrayExpression") {
                  let index = parent
                    .get_named_property::<Vec<Object>>("elements")?
                    .iter()
                    .position(|element| context.env.strict_equals(*element, *child).unwrap())
                    .unwrap();
                  if child
                    .get_named_property::<String>("type")?
                    .eq("SpreadElement")
                  {
                    path += &format!(".slice({index})");
                  } else {
                    path += &format!("[{index}]");
                  }
                } else if parent_type == "ObjectExpression" && child_type == "SpreadElement" {
                  helper = Some(context.helper("getRestElement"));
                  helper_args = Some(format!(
                    "[{}]",
                    parent
                      .get_named_property::<Vec<Object>>("properties")?
                      .iter()
                      .filter_map(|p| {
                        if p
                          .get_named_property::<String>("type")
                          .unwrap()
                          .eq("Property")
                        {
                          Some(
                            if is_string_literal(p.get_named_property::<Object>("key").ok()) {
                              format!(
                                "\"{}\"",
                                p.get_named_property::<Object>("key")
                                  .unwrap()
                                  .get_named_property::<String>("value")
                                  .unwrap()
                              )
                            } else {
                              format!(
                                "\"{}\"",
                                p.get_named_property::<Object>("key")
                                  .unwrap()
                                  .get_named_property::<String>("name")
                                  .unwrap()
                              )
                            },
                          )
                        } else {
                          None
                        }
                      })
                      .collect::<Vec<_>>()
                      .join(", ")
                  ))
                }
              }
              map.insert(
                id.get_named_property::<String>("name")?,
                Some((path, helper, helper_args)),
              );
            }
            Ok(())
          },
          true,
          None,
          None,
        )?;
      } else {
        map.insert(raw_value.clone(), None);
      }
    }
    map
  };

  let (depth, exit_scope) = context.enter_scope();
  let mut id_map = HashMap::new();
  let item_var = format!("_for_item{depth}");
  id_map.insert(item_var.clone(), String::new());

  for (id, path_info) in &id_to_path_map {
    let mut path = format!(
      "{}.value{}",
      item_var,
      if let Some(path_info) = &path_info {
        path_info.0.as_str()
      } else {
        ""
      }
    );
    if let Some((_, helper, helper_args)) = path_info
      && let Some(helper) = helper
    {
      id_map.insert(helper.clone(), String::new());
      path = format!("{helper}({path}, {})", helper_args.clone().unwrap());
    }
    id_map.insert(id.to_string(), path);
  }

  let mut args: Vec<CodeFragment> = vec![Either3::C(Some(item_var))];
  if let Some(raw_key) = raw_key.clone() {
    let key_var = format!("_for_key{depth}");
    args.push(Either3::C(Some(format!(", {key_var}"))));
    id_map.insert(raw_key, format!("{key_var}.value"));
    id_map.insert(key_var.to_string(), String::new());
  }
  if let Some(raw_index) = raw_index.clone() {
    let index_var = format!("_for_index{depth}");
    args.push(Either3::C(Some(format!(", {index_var}"))));
    id_map.insert(raw_index, format!("{index_var}.value"));
    id_map.insert(index_var.to_string(), String::new());
  }

  let (selector_patterns, key_only_binding_patterns) = match_patterns(
    context.env,
    &mut render,
    &key_prop,
    &mut id_map,
    &context.ir.source,
  )?;
  let mut selector_declarations = vec![];
  let mut selector_setup = vec![];

  let mut i = 0;
  for (_, selector) in &selector_patterns {
    let selector_name = format!("_selector{id}_{i}");
    selector_declarations.push(Either3::C(Some(format!("let {selector_name}"))));
    selector_declarations.push(Either3::A(FragmentSymbol::Newline));
    if i == 0 {
      selector_setup.push(Either3::C(Some("({ createSelector }) => {".to_string())));
      selector_setup.push(Either3::A(FragmentSymbol::IndentStart));
    }
    selector_setup.push(Either3::A(FragmentSymbol::Newline));
    selector_setup.push(Either3::C(Some(format!("{selector_name} = "))));
    let mut body = vec![Either3::C(Some("() => ".to_string()))];
    body.extend(gen_expression(selector.clone(), context, None, None)?);
    selector_setup.extend(gen_call(
      Either::A("createSelector".to_string()),
      vec![Either4::D(body)],
    ));
    if i == selector_patterns.len() - 1 {
      selector_setup.extend(vec![
        Either3::A(FragmentSymbol::IndentEnd),
        Either3::A(FragmentSymbol::Newline),
        Either3::C(Some("}".to_string())),
      ])
    }

    i += 1;
  }

  let block_fn = context.with_id(
    move || {
      let mut frag = vec![];
      frag.push(Either3::C(Some("(".to_string())));
      frag.extend(args);
      frag.push(Either3::C(Some(") => {".to_string())));
      frag.push(Either3::A(FragmentSymbol::IndentStart));
      if selector_patterns.len() > 0 || key_only_binding_patterns.len() > 0 {
        frag.extend(gen_block_content(
          Some(render),
          context,
          context_block,
          false,
          Some(Box::new(move |context_block| {
            let mut pattern_frag: Vec<CodeFragment> = vec![];

            let mut i = 0;
            for (effect, _) in selector_patterns {
              pattern_frag.extend(vec![
                Either3::A(FragmentSymbol::Newline),
                Either3::C(Some(format!("_selector{id}_{i}(() => {{"))),
                Either3::A(FragmentSymbol::IndentStart),
              ]);
              for oper in effect.operations {
                pattern_frag.extend(gen_operation(oper, context, context_block, &vec![]).unwrap());
              }
              pattern_frag.extend(vec![
                Either3::A(FragmentSymbol::IndentEnd),
                Either3::A(FragmentSymbol::Newline),
                Either3::C(Some("})".to_string())),
              ]);
              i += 1;
            }

            for effect in key_only_binding_patterns {
              for oper in effect.operations {
                pattern_frag.extend(gen_operation(oper, context, context_block, &vec![]).unwrap())
              }
            }
            pattern_frag
          })),
        )?)
      } else {
        frag.extend(gen_block_content(Some(render), context, context_block, false, None).unwrap())
      }
      frag.extend(vec![
        Either3::A(FragmentSymbol::IndentEnd),
        Either3::A(FragmentSymbol::Newline),
        Either3::C(Some("}".to_string())),
      ]);
      Ok(frag)
    },
    &id_map,
  )?;
  exit_scope();

  let mut flags = 0;
  if only_child {
    flags |= VaporVForFlags::FastRemove as i32;
  }
  if component {
    flags |= VaporVForFlags::IsComponent as i32;
  }
  if once {
    flags |= VaporVForFlags::Once as i32;
  }

  let gen_callback = move |expr: Option<SimpleExpressionNode>| {
    let Some(expr) = expr else {
      return Either4::C(None);
    };

    let mut id_map = HashMap::new();
    if let Some(raw_key) = raw_key.clone() {
      id_map.insert(raw_key, String::new());
    }
    if let Some(raw_index) = raw_index.clone() {
      id_map.insert(raw_index, String::new());
    }
    for (id, _) in id_to_path_map {
      id_map.insert(id, String::new());
    }

    let res = context
      .with_id(|| gen_expression(expr, context, None, None), &id_map)
      .unwrap();

    let mut frags = gen_multi(
      (
        Either4::C(Some(String::from("("))),
        Either4::C(Some(String::from(")"))),
        Either4::C(Some(String::from(", "))),
        None,
      ),
      vec![
        Either4::C(if !raw_value.is_empty() {
          Some(raw_value)
        } else if raw_key.is_some() || raw_index.is_some() {
          Some("_".to_string())
        } else {
          None
        }),
        Either4::C(if raw_key.is_some() {
          raw_key
        } else if raw_index.is_some() {
          Some("__".to_string())
        } else {
          None
        }),
        Either4::C(raw_index),
      ],
    );
    frags.push(Either3::C(Some(" => (".to_string())));
    frags.extend(res);
    frags.push(Either3::C(Some(")".to_string())));
    Either4::D(frags)
  };

  let mut frags = vec![Either3::A(FragmentSymbol::Newline)];
  frags.extend(selector_declarations);
  frags.push(Either3::C(Some(format!("const n{id} = "))));
  frags.extend(gen_call(
    Either::B((
      context.helper("createFor"),
      Some(Either3::C(Some("void 0".to_string()))),
    )),
    vec![
      Either4::D(source_expr),
      Either4::D(block_fn),
      gen_callback(key_prop),
      Either4::C(if flags > 0 {
        Some(flags.to_string())
      } else {
        None
      }),
      if selector_setup.len() > 0 {
        Either4::D(selector_setup)
      } else {
        Either4::C(None)
      },
      // todo: hydrationNode
    ],
  ));
  Ok(frags)
}

fn match_patterns(
  env: Env,
  render: &mut BlockIRNode,
  key_prop: &Option<SimpleExpressionNode>,
  id_map: &mut HashMap<String, String>,
  source: &str,
) -> Result<(Vec<(IREffect, SimpleExpressionNode)>, Vec<IREffect>)> {
  let mut selector_patterns = vec![];
  let mut key_only_binding_patterns = vec![];

  if let Some(key_prop) = key_prop {
    let effects = &mut render.effect;
    for i in 0..effects.len() {
      let effect = effects.get(i).unwrap();
      if let Some(selector) =
        match_selector_pattern(env, &effect, &key_prop.content, id_map, &source).unwrap()
      {
        selector_patterns.push((effects.remove(i), selector));
      } else if effect.operations.len() > 0 {
        if let Some(ast) = get_expression(&effect).unwrap().ast
          && is_key_only_binding(ast, &key_prop.content, &source)
        {
          key_only_binding_patterns.push(effects.remove(i));
        }
      }
    }
  }

  Ok((selector_patterns, key_only_binding_patterns))
}

fn match_selector_pattern(
  env: Env,
  effect: &IREffect,
  key: &str,
  id_map: &mut HashMap<String, String>,
  source: &str,
) -> Result<Option<SimpleExpressionNode>> {
  if effect.operations.len() != 1 {
    return Ok(None);
  }
  let expression = get_expression(effect);
  let Some(expression) = expression else {
    return Ok(None);
  };
  let Some(ast) = expression.ast else {
    return Ok(None);
  };

  let offset = ast.get_named_property::<u32>("start")?;

  let mut matcheds: Vec<(Object, Object)> = vec![];

  {
    let matcheds = &mut matcheds;
    SyncWalker::new(
      Some(Box::new(move |node, _, _, _| {
        if node
          .get_named_property::<String>("type")?
          .eq("BinaryExpression")
          && node.get_named_property::<String>("operator")?.eq("===")
        {
          let left = node.get_named_property::<Object>("left")?;
          let right = node.get_named_property::<Object>("right")?;
          let left_is_key = is_key_only_binding(left, key, source);
          let right_is_key = is_key_only_binding(right, key, source);
          if left_is_key && !right_is_key && analyze_variable_scopes(env, right, &id_map).len() == 0
          {
            matcheds.push((left, right));
          } else if right_is_key
            && !left_is_key
            && analyze_variable_scopes(env, left, &id_map).len() == 0
          {
            matcheds.push((right, left));
          }
        }
        Ok(None)
      })),
      None,
    )
    .visit(ast, None, None, None)?;
  }

  if matcheds.len() == 1 {
    let (key, selector) = matcheds[0];

    let mut has_extra_id = false;
    _walk_identifiers(
      env,
      ast,
      |id, _, _, _, _| {
        let start = id.get_named_property::<u32>("start")?;
        if start != key.get_named_property::<u32>("start")?
          && start != selector.get_named_property::<u32>("start")?
        {
          has_extra_id = true
        }
        Ok(())
      },
      false,
      None,
      None,
    )?;

    if !has_extra_id {
      let content = expression.content[(selector.get_named_property::<u32>("start")? - offset)
        as usize
        ..(selector.get_named_property::<u32>("end")? - offset) as usize]
        .to_string();
      return Ok(Some(SimpleExpressionNode {
        content,
        ast: Some(selector),
        loc: None,
        is_static: false,
      }));
    }
  }

  Ok(None)
}

fn analyze_variable_scopes(
  env: Env,
  ast: Object<'static>,
  id_map: &HashMap<String, String>,
) -> Vec<String> {
  let mut locals = vec![];

  {
    let locals = &mut locals;
    _walk_identifiers(
      env,
      ast,
      move |id, _, _, _, _| {
        let name = id.get_named_property::<String>("name")?;
        if !is_globally_allowed(&name) {
          if id_map.get(&name).is_some() {
            locals.push(name);
          }
        }
        Ok(())
      },
      false,
      None,
      None,
    )
    .unwrap();
  }

  return locals;
}

fn is_key_only_binding(expr: Object<'static>, key: &str, source: &str) -> bool {
  let mut _only = true;
  SyncWalker::new(
    Some(Box::new(|node, _, _, _| {
      let start = node.get_named_property::<u32>("start")? as usize;
      let end = node.get_named_property::<u32>("end")? as usize;
      if source[start..end].to_string() == key {
        return Ok(Some(Either::A(true)));
      }
      if node.get_named_property::<String>("type")?.eq("Identifier") {
        _only = false;
      };
      Ok(None)
    })),
    None,
  )
  .visit(expr, None, None, None)
  .unwrap();
  _only
}

fn get_expression(effect: &IREffect) -> Option<&SimpleExpressionNode> {
  let operation = effect.operations.get(0);
  match operation.as_ref().unwrap() {
    Either16::C(operation) => operation.values.get(0),
    Either16::G(operation) => operation.values.get(0),
    Either16::K(operation) => operation.values.get(0),
    Either16::I(operation) => Some(&operation.value),
    Either16::H(operation) => operation.value.as_ref(),
    Either16::F(operation) => Some(&operation.value),
    Either16::J(operation) => Some(&operation.value),
    Either16::D(operation) => operation.prop.values.get(0),
    _ => None,
  }
}
