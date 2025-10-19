use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::LazyLock};

use napi::{
  Either, Env, Result,
  bindgen_prelude::{Either3, Either18, FnArgs, Function, JsObjectValue, Object},
};
use napi_derive::napi;

use crate::{
  ir::{
    component::{
      IRDynamicPropsKind, IRProp, IRProps, IRPropsDynamicAttribute, IRPropsDynamicExpression,
      IRPropsStatic, IRSlots,
    },
    index::{
      CreateComponentIRNode, DirectiveIRNode, DynamicFlag, IRNodeTypes, SetDynamicEventsIRNode,
      SetDynamicPropsIRNode, SetPropIRNode, SimpleExpressionNode,
    },
  },
  transform::{
    DirectiveTransformResult, is_operation, push_template, reference, register_effect,
    register_operation, v_bind::transform_v_bind, v_html::transform_v_html, v_if::transform_v_if,
    v_model::transform_v_model, v_on::transform_v_on, v_show::transform_v_show,
    v_slots::transform_v_slots, v_text::transform_v_text,
  },
  utils::{
    check::{is_build_in_directive, is_jsx_component, is_template, is_void_tag},
    directive::resolve_directive,
    dom::is_valid_html_nesting,
    error::{ErrorCodes, on_error},
    expression::{create_simple_expression, resolve_expression},
    text::{camelize, get_text},
    utils::get_text_like_value,
  },
};

static RESERVED_PROP: [&str; 5] = ["", "key", "ref", "ref_for", "ref_key"];
pub fn is_reserved_prop(name: &str) -> bool {
  RESERVED_PROP.contains(&name)
}

static IS_EVENT_REGEX: LazyLock<regex::Regex> =
  LazyLock::new(|| regex::Regex::new(r"^on[A-Z]").unwrap());

static IS_DIRECTIVE_REGEX: LazyLock<regex::Regex> =
  LazyLock::new(|| regex::Regex::new(r"^v-[a-z]").unwrap());

pub fn transform_element(
  env: Env,
  _: Object<'static>,
  context: Object<'static>,
) -> Result<Option<Box<dyn FnOnce() -> Result<()>>>> {
  let block = context.get_named_property::<Object>("block")?;
  let mut effect_index = block.get_named_property::<Vec<Object>>("effect")?.len() as i32;
  let get_effect_index = Rc::new(RefCell::new(Box::new(move || {
    let current = effect_index;
    effect_index += 1;
    current
  }) as Box<dyn FnMut() -> i32>));
  let mut operation_index = block.get_named_property::<Vec<Object>>("operation")?.len() as i32;
  let get_operation_index = Rc::new(RefCell::new(Box::new(move || {
    let current = operation_index;
    operation_index += 1;
    current
  }) as Box<dyn FnMut() -> i32>));
  Ok(Some(Box::new(move || {
    let node = context.get_named_property::<Object>("node")?;
    if !node.get_named_property::<String>("type")?.eq("JSXElement") || is_template(Some(node)) {
      return Ok(());
    }

    let name = node
      .get_named_property::<Object>("openingElement")?
      .get_named_property::<Object>("name")?;
    let tag = get_text(name, context);
    let is_component = is_jsx_component(node);
    let props_result = build_props(
      env,
      node,
      context,
      is_component,
      Rc::clone(&get_effect_index),
      Rc::clone(&get_operation_index),
    )?;
    let mut parent = context.get_named_property::<Object>("parent");
    while let Ok(_parent) = parent
      && _parent
        .get_named_property::<Object>("node")?
        .get_named_property::<String>("type")?
        .eq("JSXElement")
      && is_template(_parent.get_named_property::<Object>("node").ok())
      && let Ok(_parent) = _parent.get_named_property::<Object>("parent")
    {
      parent = Ok(_parent)
    }
    let single_root = if let Ok(parent) = parent
      && env.strict_equals(context.get_named_property::<Object>("root")?, parent)?
      && !parent
        .get_named_property::<Object>("node")?
        .get_named_property::<String>("type")?
        .eq("JSXFragment")
    {
      true
    } else {
      false
    };

    if is_component {
      transform_component_element(env, tag, props_result, single_root, context)?;
    } else {
      transform_native_element(
        tag,
        props_result,
        single_root,
        context,
        Rc::clone(&get_effect_index),
        Rc::clone(&get_operation_index),
      )?;
    }

    Ok(())
  })))
}

pub fn transform_native_element<'a>(
  tag: String,
  props_result: PropsResult,
  single_root: bool,
  mut context: Object,
  get_effect_index: Rc<RefCell<Box<dyn FnMut() -> i32>>>,
  get_operation_index: Rc<RefCell<Box<dyn FnMut() -> i32>>>,
) -> Result<()> {
  let mut template = format!("<{tag}");

  let mut dynamic_props = vec![];

  match props_result.props {
    Either::A(props) => {
      /* dynamic props */
      register_effect(
        &context,
        false,
        Either18::E(SetDynamicPropsIRNode {
          _type: IRNodeTypes::SET_DYNAMIC_PROPS,
          element: reference(context)?,
          props,
          root: single_root,
        }),
        Some(get_effect_index),
        Some(get_operation_index),
      )?
    }
    Either::B(props) => {
      for prop in props {
        let key = &prop.key;
        let values = &prop.values;
        if key.is_static && values.len() == 1 && values[0].is_static {
          template += &format!(" {}", key.content);
          if !values[0].content.is_empty() {
            template += &format!("=\"{}\"", values[0].content);
          }
        } else {
          dynamic_props.push(key.content.clone());

          register_effect(
            &context,
            is_operation(
              values.iter().collect::<Vec<&SimpleExpressionNode>>(),
              &context,
            ),
            Either18::D(SetPropIRNode {
              _type: IRNodeTypes::SET_PROP,
              element: reference(context)?,
              prop,
              tag: tag.clone(),
              root: single_root,
            }),
            Some(Rc::clone(&get_effect_index)),
            Some(Rc::clone(&get_operation_index)),
          )?;
        }
      }
    }
  }

  template += &format!(
    ">{}",
    context
      .get_named_property::<Vec<String>>("childrenTemplate")?
      .join("")
  );
  // TODO remove unnecessary close tag, e.g. if it's the last element of the template
  if !is_void_tag(&tag) {
    template += &format!("</{}>", tag)
  }

  if single_root {
    let mut ir = context.get_named_property::<Object>("ir")?;
    ir.set(
      "rootTemplateIndex",
      ir.get_named_property::<Vec<String>>("templates")?.len() as i32,
    )?;
  }

  let parent = context.get_named_property::<Object>("parent");
  if let Ok(parent) = parent
    && let Ok(node) = parent.get_named_property::<Object>("node")
    && node.get_named_property::<String>("type")?.eq("JSXElement")
    && let Ok(name) = node
      .get_named_property::<Object>("openingElement")?
      .get_named_property::<Object>("name")
    && name
      .get_named_property::<String>("type")?
      .eq("JSXIdentifier")
    && !is_valid_html_nesting(&name.get_named_property::<String>("name")?, &tag)
  {
    reference(context)?;
    let mut dynamic = context
      .get_named_property::<Object>("block")?
      .get_named_property::<Object>("dynamic")?;
    dynamic.set("template", push_template(context, template)?)?;
    dynamic.set(
      "flags",
      dynamic.get_named_property::<i32>("flags")?
        | DynamicFlag::NON_TEMPLATE as i32
        | DynamicFlag::INSERT as i32,
    )?;
  } else {
    context.set(
      "template",
      format!(
        "{}{}",
        context.get_named_property::<String>("template")?,
        template
      ),
    )?
  }
  Ok(())
}

#[napi]
pub fn transform_component_element(
  env: Env,
  mut tag: String,
  props_result: PropsResult,
  single_root: bool,
  mut context: Object,
) -> Result<()> {
  let mut asset = context
    .get_named_property::<Object>("options")?
    .get_named_property::<bool>("withFallback")?;

  if let Some(dot_index) = tag.find('.') {
    let ns = tag[0..dot_index].to_string();
    if !ns.is_empty() {
      tag = ns + &tag[dot_index..].to_string();
    }
  }

  if tag.contains("-") {
    asset = true
  }

  if asset {
    let component = context
      .get_named_property::<Object>("ir")?
      .get_named_property::<Object>("component")?;
    component
      .get_named_property::<Function<String, Object>>("add")?
      .apply(component, tag.clone())?;
  }

  let mut dynamic = context
    .get_named_property::<Object>("block")?
    .get_named_property::<Object>("dynamic")?;
  dynamic.set(
    "flags",
    dynamic.get_named_property::<i32>("flags")?
      | DynamicFlag::NON_TEMPLATE as i32
      | DynamicFlag::INSERT as i32,
  )?;

  dynamic.set(
    "operation",
    CreateComponentIRNode {
      _type: IRNodeTypes::CREATE_COMPONENT_NODE,
      id: reference(context)?,
      tag,
      props: match props_result.props {
        Either::A(props) => props,
        Either::B(props) => vec![Either3::A(props)],
      },
      asset,
      root: single_root && context.get_named_property::<i32>("inVFor")? == 0,
      slots: context.get_named_property::<Vec<IRSlots>>("slots")?,
      once: context.get_named_property::<bool>("inVOnce")?,
      parent: None,
      anchor: None,
      dynamic: None,
    },
  )?;

  context.set("slots", env.create_array(0))?;

  Ok(())
}

#[napi(object)]
pub struct PropsResult {
  pub dynamic: bool,
  pub props: Either<Vec<IRProps>, IRPropsStatic>,
}

pub fn build_props(
  env: Env,
  node: Object,
  context: Object,
  is_component: bool,
  get_effect_index: Rc<RefCell<Box<dyn FnMut() -> i32>>>,
  get_operation_index: Rc<RefCell<Box<dyn FnMut() -> i32>>>,
) -> Result<PropsResult> {
  let props = node
    .get_named_property::<Object>("openingElement")?
    .get_named_property::<Vec<Object>>("attributes")?;
  if props.is_empty() {
    return Ok(PropsResult {
      dynamic: false,
      props: Either::B(vec![]),
    });
  }

  let mut dynamic_args: Vec<IRProps> = vec![];
  let mut results: Vec<DirectiveTransformResult> = vec![];

  for prop in props {
    if prop
      .get_named_property::<String>("type")?
      .eq("JSXSpreadAttribute")
      && let Ok(argument) = prop.get_named_property::<Object>("argument")
    {
      let value = resolve_expression(argument, context);
      if !results.is_empty() {
        dynamic_args.push(Either3::A(dedupe_properties(results)));
        results = vec![];
      }
      dynamic_args.push(Either3::C(IRPropsDynamicExpression {
        kind: IRDynamicPropsKind::EXPRESSION,
        value,
        handler: None,
      }));
      continue;
    }

    let prop_name = get_text(prop.get_named_property::<Object>("name")?, context);
    if prop_name == "v-on" {
      // v-on={obj}
      if let Ok(prop_value) = prop.get_named_property::<Object>("value") {
        let value = resolve_expression(prop_value, context);
        if is_component {
          if !results.is_empty() {
            dynamic_args.push(Either3::A(dedupe_properties(results)));
            results = vec![];
          }
          dynamic_args.push(Either3::C(IRPropsDynamicExpression {
            kind: IRDynamicPropsKind::EXPRESSION,
            value,
            handler: Some(true),
          }))
        } else {
          register_effect(
            &context,
            is_operation(vec![&value], &context),
            Either18::F(SetDynamicEventsIRNode {
              _type: IRNodeTypes::SET_DYNAMIC_EVENTS,
              element: reference(context)?,
              value,
            }),
            Some(Rc::clone(&get_effect_index)),
            Some(Rc::clone(&get_operation_index)),
          )?;
        }
      } else {
        on_error(env, ErrorCodes::X_V_ON_NO_EXPRESSION, context);
      }
      continue;
    }

    if let Some(prop) = transform_prop(
      env,
      prop,
      node,
      is_component,
      context,
      Rc::clone(&get_operation_index),
    )? {
      if is_component && !prop.key.is_static {
        // v-model:&name&="value"
        if !results.is_empty() {
          dynamic_args.push(Either3::A(dedupe_properties(results)));
          results = vec![];
        }
        dynamic_args.push(Either3::B(IRPropsDynamicAttribute {
          kind: IRDynamicPropsKind::ATTRIBUTE,
          key: prop.key,
          modifier: prop.modifier,
          runtime_camelize: prop.runtime_camelize,
          handler: prop.handler,
          handler_modifiers: prop.handler_modifiers,
          model: prop.model,
          model_modifiers: prop.model_modifiers,
          values: vec![prop.value],
        }));
      } else {
        // other static props
        results.push(prop)
      }
    }
  }

  // has dynamic key or {...obj}
  if !dynamic_args.is_empty() || results.iter().any(|prop| !prop.key.is_static) {
    // take rest of props as dynamic props
    if !results.is_empty() {
      dynamic_args.push(Either3::A(dedupe_properties(results)));
    }
    return Ok(PropsResult {
      dynamic: true,
      props: Either::A(dynamic_args),
    });
  }

  Ok(PropsResult {
    dynamic: false,
    props: Either::B(dedupe_properties(results)),
  })
}

pub fn transform_prop(
  env: Env,
  prop: Object,
  node: Object,
  is_component: bool,
  context: Object,
  get_operation_index: Rc<RefCell<Box<dyn FnMut() -> i32>>>,
) -> Result<Option<DirectiveTransformResult>> {
  let prop_type = prop.get_named_property::<String>("type")?;
  if prop_type == "JSXSpreadAttribute" {
    return Ok(None);
  }
  let prop_name = prop.get_named_property::<Object>("name")?;
  let name_type = prop_name.get_named_property::<String>("type")?;
  let name = if name_type == "JSXIdentifier" {
    prop_name.get_named_property::<String>("name")?
  } else if name_type == "JSXNamespacedName" {
    prop_name
      .get_named_property::<Object>("namespace")?
      .get_named_property::<String>("name")?
  } else {
    return Ok(None);
  };
  let name = name.split("_").collect::<Vec<&str>>()[0];
  let prop_value = prop.get_named_property::<Object>("value");
  let value = if let Ok(prop_value) = prop_value {
    get_text_like_value(prop_value, Some(is_component))
  } else {
    None
  };
  if !IS_DIRECTIVE_REGEX.is_match(&name)
    && !IS_EVENT_REGEX.is_match(&name)
    && (prop_value.is_err() || value.is_some())
  {
    if is_reserved_prop(name) {
      return Ok(None);
    }
    return Ok(Some(DirectiveTransformResult::new(
      create_simple_expression(name.to_string(), Some(true), Some(prop_name), None),
      if let Some(value) = value {
        create_simple_expression(value, Some(true), Some(prop_name), None)
      } else {
        create_simple_expression("true".to_string(), Some(false), None, None)
      },
    )));
  }

  let mut name = if IS_EVENT_REGEX.is_match(&name) {
    "on".to_string()
  } else if IS_DIRECTIVE_REGEX.is_match(&name) {
    name[2..].to_string()
  } else {
    "bind".to_string()
  };
  let options = context.get_named_property::<Object>("options")?;
  let directive_transforms =
    options.get_named_property::<HashMap<
      String,
      Function<FnArgs<(Object, Object, Object)>, Option<DirectiveTransformResult>>,
    >>("directiveTransforms");

  match name.as_str() {
    "bind" => return transform_v_bind(prop, node, context),
    "on" => return transform_v_on(env, prop, node, context),
    "model" => return transform_v_model(env, prop, node, context),
    "show" => return transform_v_show(env, prop, node, context),
    "html" => return transform_v_html(env, prop, node, context),
    "text" => return transform_v_text(env, prop, node, context),
    "slots" => return transform_v_slots(env, prop, node, context),
    _ => (),
  };

  if let Ok(directive_transforms) = directive_transforms
    && let Some(directive_transform) = directive_transforms.get(&name)
  {
    return Ok(directive_transform.call(FnArgs::from((prop, node, context)))?);
  }

  if !is_build_in_directive(&name) {
    let with_fallback = options.get_named_property::<bool>("withFallback")?;
    if with_fallback {
      let directive = context
        .get_named_property::<Object>("ir")?
        .get_named_property::<Object>("directive")?;
      directive
        .get_named_property::<Function<String, Object>>("add")?
        .apply(directive, name.clone())?;
    } else {
      name = camelize(format!("v-{name}"))
    };

    register_operation(
      &context,
      Either18::N(DirectiveIRNode {
        _type: IRNodeTypes::DIRECTIVE,
        element: reference(context)?,
        dir: resolve_directive(prop, context)?,
        name,
        asset: Some(with_fallback),
        builtin: None,
        model_type: None,
      }),
      Some(Rc::clone(&get_operation_index)),
    )?
  }

  Ok(None)
}

// Dedupe props in an object literal.
// Literal duplicated attributes would have been warned during the parse phase,
// however, it's possible to encounter duplicated `onXXX` handlers with different
// modifiers. We also need to merge static and dynamic class / style attributes.
pub fn dedupe_properties(results: Vec<DirectiveTransformResult>) -> Vec<IRProp> {
  let mut deduped = vec![];

  for result in results {
    let prop = IRProp {
      key: result.key,
      modifier: result.modifier,
      runtime_camelize: result.runtime_camelize,
      handler: result.handler,
      handler_modifiers: result.handler_modifiers,
      model: result.model,
      model_modifiers: result.model_modifiers,
      values: vec![result.value],
    };
    // dynamic keys are always allowed
    if !prop.key.is_static {
      deduped.push(prop);
      continue;
    }
    let name = prop.key.content.as_str();
    let existing = deduped.iter_mut().find(|i| i.key.content == name);
    if let Some(existing) = existing {
      if name == "style" || name == "class" {
        let existing = existing;
        for value in prop.values {
          existing.values.push(value)
        }
      }
    // unexpected duplicate, should have emitted error during parse
    } else {
      deduped.push(prop);
    }
  }
  return deduped;
}
