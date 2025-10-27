use std::{cell::RefCell, rc::Rc, sync::LazyLock};

use napi::{
  Either, Result,
  bindgen_prelude::{Either3, Either16, JsObjectValue, Object},
};
use napi_derive::napi;

use crate::{
  ir::{
    component::{
      IRDynamicPropsKind, IRProp, IRProps, IRPropsDynamicAttribute, IRPropsDynamicExpression,
      IRPropsStatic,
    },
    index::{
      BlockIRNode, CreateComponentIRNode, DirectiveIRNode, DynamicFlag, IRDynamicInfo, IRNodeTypes,
      SetDynamicEventsIRNode, SetDynamicPropsIRNode, SetPropIRNode, SimpleExpressionNode,
    },
  },
  transform::{
    DirectiveTransformResult, TransformContext, v_bind::transform_v_bind, v_html::transform_v_html,
    v_model::transform_v_model, v_on::transform_v_on, v_show::transform_v_show,
    v_slots::transform_v_slots, v_text::transform_v_text,
  },
  utils::{
    check::{is_build_in_directive, is_jsx_component, is_template, is_void_tag},
    directive::resolve_directive,
    dom::is_valid_html_nesting,
    error::{ErrorCodes, on_error},
    expression::{create_simple_expression, resolve_expression},
    my_box::MyBox,
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

pub fn transform_element<'a>(
  node: Object<'static>,
  context: &'a Rc<TransformContext>,
  context_block: &'a mut BlockIRNode,
  _: &'a mut IRDynamicInfo,
) -> Result<Option<Box<dyn FnOnce() -> Result<()> + 'a>>> {
  let mut effect_index = context_block.effect.len() as i32;
  let get_effect_index = Rc::new(RefCell::new(Box::new(move || {
    let current = effect_index;
    effect_index += 1;
    current
  }) as Box<dyn FnMut() -> i32>));
  let mut operation_index = context_block.operation.len() as i32;
  let get_operation_index = Rc::new(RefCell::new(Box::new(move || {
    let current = operation_index;
    operation_index += 1;
    current
  }) as Box<dyn FnMut() -> i32>));
  Ok(Some(Box::new(move || {
    if !node.get_named_property::<String>("type")?.eq("JSXElement") || is_template(&node) {
      return Ok(());
    }

    let name = node
      .get_named_property::<Object>("openingElement")?
      .get_named_property::<Object>("name")?;
    let tag = get_text(name, context);
    let is_component = is_jsx_component(node);
    let props_result = build_props(
      node,
      context,
      context_block,
      is_component,
      Rc::clone(&get_effect_index),
      Rc::clone(&get_operation_index),
    )?;
    let mut parent = context.parent.borrow_mut().upgrade();
    while let Some(ref _parent) = parent {
      if _parent.node.borrow().get_named_property::<String>("type")? != "JSXElement"
        || !is_template(&_parent.node.borrow())
      {
        break;
      }
      let next_parent = _parent.parent.borrow().upgrade();
      if next_parent.is_none() {
        break;
      }
      parent = next_parent;
    }
    let single_root = if let Some(parent) = parent
      && Rc::ptr_eq(&context.root.borrow().upgrade().unwrap(), &parent)
      && !parent
        .node
        .borrow()
        .get_named_property::<String>("type")?
        .eq("JSXFragment")
    {
      true
    } else {
      false
    };

    if is_component {
      transform_component_element(tag, props_result, single_root, context, context_block)?;
    } else {
      transform_native_element(
        tag,
        props_result,
        single_root,
        context,
        context_block,
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
  context: &'a Rc<TransformContext>,
  context_block: &'a mut BlockIRNode,
  get_effect_index: Rc<RefCell<Box<dyn FnMut() -> i32 + 'a>>>,
  get_operation_index: Rc<RefCell<Box<dyn FnMut() -> i32 + 'a>>>,
) -> Result<()> {
  let mut template = format!("<{tag}");

  let mut dynamic_props = vec![];

  match props_result.props {
    Either::A(props) => {
      let element = context.reference(&mut context_block.dynamic)?;
      /* dynamic props */
      context.register_effect(
        context_block,
        false,
        Either16::E(SetDynamicPropsIRNode {
          _type: IRNodeTypes::SET_DYNAMIC_PROPS,
          props,
          element,
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

          let element = context.reference(&mut context_block.dynamic)?;
          context.register_effect(
            context_block,
            context.is_operation(values.iter().collect::<Vec<&SimpleExpressionNode>>()),
            Either16::D(SetPropIRNode {
              _type: IRNodeTypes::SET_PROP,
              prop,
              element,
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

  template += &format!(">{}", context.children_template.borrow().join(""));
  // TODO remove unnecessary close tag, e.g. if it's the last element of the template
  if !is_void_tag(&tag) {
    template += &format!("</{}>", tag)
  }

  if single_root {
    let ir = &mut context.ir.borrow_mut();
    ir.root_template_index = Some(ir.templates.len() as i32)
  }

  let parent = context.parent.borrow().upgrade();
  if let Some(parent) = parent
    && let node = parent.node.borrow()
    && node.get_named_property::<String>("type")?.eq("JSXElement")
    && let Ok(name) = node
      .get_named_property::<Object>("openingElement")?
      .get_named_property::<Object>("name")
    && name
      .get_named_property::<String>("type")?
      .eq("JSXIdentifier")
    && !is_valid_html_nesting(&name.get_named_property::<String>("name")?, &tag)
  {
    let dynamic = &mut context_block.dynamic;
    context.reference(dynamic)?;
    dynamic.template = Some(context.push_template(template)?);
    dynamic.flags = dynamic.flags | DynamicFlag::NON_TEMPLATE as i32 | DynamicFlag::INSERT as i32;
  } else {
    *context.template.borrow_mut() = format!("{}{}", context.template.borrow(), template);
  }
  Ok(())
}

pub fn transform_component_element(
  mut tag: String,
  props_result: PropsResult,
  single_root: bool,
  context: &Rc<TransformContext>,
  context_block: &mut BlockIRNode,
) -> Result<()> {
  let mut asset = context.options.with_fallback;

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
    let component = &mut context.ir.borrow_mut().component;
    component.insert(tag.clone());
  }

  let dynamic = &mut context_block.dynamic;
  dynamic.flags = dynamic.flags | DynamicFlag::NON_TEMPLATE as i32 | DynamicFlag::INSERT as i32;

  dynamic.operation = Some(MyBox(Box::new(Either16::N(CreateComponentIRNode {
    _type: IRNodeTypes::CREATE_COMPONENT_NODE,
    id: context.reference(dynamic)?,
    tag,
    props: match props_result.props {
      Either::A(props) => props,
      Either::B(props) => vec![Either3::A(props)],
    },
    asset,
    root: single_root && *context.in_v_for.borrow() == 0,
    slots: context.slots.take(),
    once: *context.in_v_once.borrow(),
    parent: None,
    anchor: None,
    dynamic: None,
  }))));

  Ok(())
}

#[napi(object)]
pub struct PropsResult {
  pub dynamic: bool,
  pub props: Either<Vec<IRProps>, IRPropsStatic>,
}

pub fn build_props<'a>(
  node: Object,
  context: &'a Rc<TransformContext>,
  context_block: &mut BlockIRNode,
  is_component: bool,
  get_effect_index: Rc<RefCell<Box<dyn FnMut() -> i32 + 'a>>>,
  get_operation_index: Rc<RefCell<Box<dyn FnMut() -> i32 + 'a>>>,
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
          let element = context.reference(&mut context_block.dynamic)?;
          context.register_effect(
            context_block,
            context.is_operation(vec![&value]),
            Either16::F(SetDynamicEventsIRNode {
              _type: IRNodeTypes::SET_DYNAMIC_EVENTS,
              element,
              value,
            }),
            Some(Rc::clone(&get_effect_index)),
            Some(Rc::clone(&get_operation_index)),
          )?;
        }
      } else {
        on_error(ErrorCodes::X_V_ON_NO_EXPRESSION, context);
      }
      continue;
    }

    if let Some(prop) = transform_prop(
      prop,
      node,
      is_component,
      context,
      context_block,
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

pub fn transform_prop<'a>(
  prop: Object,
  node: Object,
  is_component: bool,
  context: &'a Rc<TransformContext>,
  context_block: &mut BlockIRNode,
  get_operation_index: Rc<RefCell<Box<dyn FnMut() -> i32 + 'a>>>,
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

  match name.as_str() {
    "bind" => return transform_v_bind(prop, node, context, context_block),
    "on" => return transform_v_on(prop, node, context, context_block),
    "model" => return transform_v_model(prop, node, context, context_block),
    "show" => return transform_v_show(prop, node, context, context_block),
    "html" => return transform_v_html(prop, node, context, context_block),
    "text" => return transform_v_text(prop, node, context, context_block),
    "slots" => return transform_v_slots(prop, node, context, context_block),
    _ => (),
  };

  if !is_build_in_directive(&name) {
    let with_fallback = context.options.with_fallback;
    if with_fallback {
      let directive = &mut context.ir.borrow_mut().directive;
      directive.insert(name.clone());
    } else {
      name = camelize(format!("v-{name}"))
    };

    let element = context.reference(&mut context_block.dynamic)?;
    context.register_operation(
      context_block,
      Either16::M(DirectiveIRNode {
        _type: IRNodeTypes::DIRECTIVE,
        element,
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
