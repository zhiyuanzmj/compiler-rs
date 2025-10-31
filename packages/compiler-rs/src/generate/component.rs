use std::mem;

use napi::Either;
use napi::Result;
use napi::bindgen_prelude::Either3;
use napi::bindgen_prelude::Either4;

use crate::generate::CodegenContext;
use crate::generate::directive::gen_directive_modifiers;
use crate::generate::directive::gen_directives_for_element;
use crate::generate::event::gen_event_handler;
use crate::generate::expression::gen_expression;
use crate::generate::prop::gen_prop_key;
use crate::generate::prop::gen_prop_value;
use crate::generate::slot::gen_raw_slots;
use crate::generate::utils::CodeFragment;
use crate::generate::utils::CodeFragments;
use crate::generate::utils::FragmentSymbol::Newline;
use crate::generate::utils::gen_call;
use crate::generate::utils::gen_multi;
use crate::generate::utils::get_delimiters_array_newline;
use crate::generate::utils::get_delimiters_object;
use crate::generate::utils::get_delimiters_object_newline;
use crate::generate::utils::to_valid_asset_id;
use crate::generate::v_model::gen_model_handler;
use crate::ir::component::IRProp;
use crate::ir::component::IRProps;
use crate::ir::component::IRPropsStatic;
use crate::ir::index::BlockIRNode;
use crate::ir::index::CreateComponentIRNode;
use crate::ir::index::SimpleExpressionNode;
use crate::utils::expression::create_simple_expression;
use crate::utils::text::camelize;

pub fn gen_create_component(
  operation: CreateComponentIRNode,
  context: &CodegenContext,
  context_block: &mut BlockIRNode,
) -> Result<Vec<CodeFragment>> {
  let CreateComponentIRNode {
    tag,
    root,
    props,
    slots,
    once,
    id,
    dynamic,
    asset,
    ..
  } = operation;

  let is_dynamic = if let Some(dynamic) = &dynamic
    && !dynamic.is_static
  {
    true
  } else {
    false
  };
  let tag: CodeFragments = if let Some(dynamic) = dynamic {
    Either4::D(if dynamic.is_static {
      gen_call(
        Either::A(context.helper("resolveDynamicComponent")),
        vec![Either4::D(gen_expression(dynamic, context, None, None)?)],
      )
    } else {
      let mut result = vec![Either3::C(Some("() => (".to_string()))];
      result.extend(gen_expression(dynamic, context, None, None)?);
      result.push(Either3::C(Some(")".to_string())));
      result
    })
  } else if asset {
    Either4::C(Some(to_valid_asset_id(tag, "component".to_string())))
  } else {
    Either4::D(gen_expression(
      create_simple_expression(tag, None, None, None),
      context,
      None,
      None,
    )?)
  };

  let raw_props = gen_raw_props(props, context)?;
  let raw_slots = gen_raw_slots(slots, context, context_block)?;

  let mut result = vec![
    Either3::A(Newline),
    Either3::C(Some(format!("const n{id} = "))),
  ];
  result.extend(gen_call(
    Either::A(context.helper(if is_dynamic {
      "createDynamicComponent"
    } else if asset {
      "createComponentWithFallback"
    } else {
      "createComponent"
    })),
    vec![
      tag,
      if let Some(raw_props) = raw_props {
        Either4::D(raw_props)
      } else {
        Either4::C(None)
      },
      if let Some(raw_slots) = raw_slots {
        Either4::D(raw_slots)
      } else {
        Either4::C(None)
      },
      Either4::C(if root { Some("true".to_string()) } else { None }),
      Either4::C(if once { Some("true".to_string()) } else { None }),
    ],
  ));
  result.extend(gen_directives_for_element(id, context, context_block)?);
  Ok(result)
}

pub fn gen_raw_props(
  mut props: Vec<IRProps>,
  context: &CodegenContext,
) -> Result<Option<Vec<CodeFragment>>> {
  let props_len = props.len();
  Ok(if let Either3::A(static_props) = &props[0] {
    if static_props.len() == 0 && props_len == 1 {
      return Ok(None);
    }
    let static_props = props.remove(0);
    if let Either3::A(static_props) = static_props {
      Some(gen_static_props(
        static_props,
        context,
        gen_dynamic_props(props, context)?,
      )?)
    } else {
      None
    }
  } else if props_len > 0 {
    // all dynamic
    Some(gen_static_props(
      vec![],
      context,
      gen_dynamic_props(props, context)?,
    )?)
  } else {
    None
  })
}

fn gen_static_props(
  props: IRPropsStatic,
  context: &CodegenContext,
  dynamic_props: Option<Vec<CodeFragment>>,
) -> Result<Vec<CodeFragment>> {
  let mut args = props
    .into_iter()
    .map(|prop| Either4::D(gen_prop(prop, context, true).unwrap()))
    .collect::<Vec<_>>();
  if let Some(dynamic_props) = dynamic_props {
    let mut result = vec![Either3::C(Some("$: ".to_string()))];
    result.extend(dynamic_props);
    args.push(Either4::D(result));
  }
  return Ok(gen_multi(
    if args.len() > 1 {
      get_delimiters_object_newline()
    } else {
      get_delimiters_object()
    },
    args,
  ));
}

fn gen_dynamic_props(
  props: Vec<IRProps>,
  context: &CodegenContext,
) -> Result<Option<Vec<CodeFragment>>> {
  let mut frags: Vec<CodeFragments> = vec![];
  for p in props {
    let mut expr = None;
    if let Either3::A(p) = p {
      if p.len() > 0 {
        frags.push(Either4::D(gen_static_props(p, context, None)?))
      }
      continue;
    } else if let Either3::B(p) = p {
      expr = Some(gen_multi(
        get_delimiters_object(),
        vec![Either4::D(gen_prop(p, context, false)?)],
      ))
    } else if let Either3::C(p) = p {
      let expression = gen_expression(p.value, context, None, None)?;
      expr = if p.handler.unwrap_or_default() {
        Some(gen_call(
          Either::A(context.helper("toHandlers")),
          vec![Either4::D(expression)],
        ))
      } else {
        Some(expression)
      }
    }
    let mut result = vec![Either3::C(Some("() => (".to_string()))];
    result.extend(expr.unwrap());
    result.push(Either3::C(Some(")".to_string())));
    frags.push(Either4::D(result));
  }
  if frags.len() > 0 {
    return Ok(Some(gen_multi(get_delimiters_array_newline(), frags)));
  }
  return Ok(None);
}

fn gen_prop(
  mut prop: IRProp,
  context: &CodegenContext,
  is_static: bool,
) -> Result<Vec<CodeFragment>> {
  let model = prop.model.unwrap_or_default();
  let handler = prop.handler.unwrap_or_default();
  let handler_modifiers = prop.handler_modifiers.clone();
  let mut values = mem::take(&mut prop.values);
  let first_value = values[0].clone();
  let cloned_key = prop.key.clone();
  let model_modifiers = prop.model_modifiers.take();
  let mut result = gen_prop_key(prop, context)?;
  result.push(Either3::C(Some(": ".to_string())));
  result.extend(if handler {
    gen_event_handler(
      context,
      Some(values.remove(0)),
      handler_modifiers,
      true, /* wrap handlers passed to components */
    )?
  } else {
    let values = gen_prop_value(values, context)?;
    if is_static {
      let mut result: Vec<CodeFragment> = vec![Either3::C(Some("() => (".to_string()))];
      result.extend(values);
      result.push(Either3::C(Some(")".to_string())));
      result
    } else {
      values
    }
  });
  if model {
    let models = gen_model(cloned_key, first_value, model_modifiers, context)?;
    result.extend(models)
  }
  Ok(result)
}

fn gen_model(
  key: SimpleExpressionNode,
  value: SimpleExpressionNode,
  model_modifiers: Option<Vec<String>>,
  context: &CodegenContext,
) -> Result<Vec<CodeFragment>> {
  let is_static = key.is_static;
  let content = key.content.clone();
  let expression = gen_expression(key, context, None, None)?;
  let name: Vec<CodeFragment> = if is_static {
    vec![Either3::C(Some(format!(
      "\"onUpdate:{}\"",
      camelize(content.clone())
    )))]
  } else {
    let mut result = vec![Either3::C(Some("[\"onUpdate:\" + ".to_string()))];
    result.extend(expression.clone());
    result.push(Either3::C(Some("]".to_string())));
    result
  };
  let handler = gen_model_handler(value, context)?;

  let mut result = vec![Either3::C(Some(",".to_string())), Either3::A(Newline)];
  result.extend(name);
  result.push(Either3::C(Some(": () => ".to_string())));
  result.extend(handler);

  if let Some(model_modifiers) = model_modifiers
    && model_modifiers.len() > 0
  {
    let modifers_key = if is_static {
      vec![Either3::C(Some(format!("{content}Modifiers")))]
    } else {
      let mut result = vec![Either3::C(Some("[".to_string()))];
      result.extend(expression);
      result.push(Either3::C(Some(" + \"Modifiers\"]".to_string())));
      result
    };
    let modifiers_val = gen_directive_modifiers(model_modifiers);
    result.extend(vec![Either3::C(Some(",".to_string())), Either3::A(Newline)]);
    result.extend(modifers_key);
    result.push(Either3::C(Some(format!(": () => ({{ {modifiers_val} }})"))));
  }
  Ok(result)
}
