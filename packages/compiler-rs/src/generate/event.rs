use napi::Either;
use napi::Env;
use napi::Result;
use napi::bindgen_prelude::Either3;
use napi::bindgen_prelude::Either4;
use napi::bindgen_prelude::Either16;
use napi::bindgen_prelude::Function;
use napi::bindgen_prelude::JsObjectValue;
use napi::bindgen_prelude::Object;
use napi_derive::napi;

use crate::generate::expression::gen_expression;
use crate::generate::utils::CodeFragment;
use crate::generate::utils::FragmentSymbol::Newline;
use crate::generate::utils::gen_call;
use crate::generate::utils::gen_multi;
use crate::generate::utils::get_delimiters_object_newline;
use crate::ir::index::Modifiers;
use crate::ir::index::OperationNode;
use crate::ir::index::SetDynamicEventsIRNode;
use crate::ir::index::SetEventIRNode;
use crate::ir::index::SimpleExpressionNode;
use crate::utils::check::is_fn_expression;
use crate::utils::check::is_member_expression;

#[napi]
pub fn gen_set_event(env: Env, oper: SetEventIRNode, context: Object) -> Result<Vec<CodeFragment>> {
  let SetEventIRNode {
    element,
    key,
    key_override,
    value,
    modifiers,
    delegate,
    effect,
    ..
  } = oper;

  let key_content = key.content.clone();
  let oper_key_strat = key.ast.unwrap().get_named_property::<u32>("start").unwrap();
  let name = if let Some(key_override) = key_override {
    let find = format!("\"{}\"", key_override.0);
    let replacement = format!("\"{}\"", key_override.1);
    let mut wrapped = vec![Either3::C(Some("(".to_string()))];
    wrapped.extend(gen_expression(env, key, context, None, None)?);
    wrapped.push(Either3::C(Some(")".to_string())));
    let cloned = wrapped.clone();
    wrapped.push(Either3::C(Some(format!(" === {find} ? {replacement} : "))));
    wrapped.extend(cloned);
    wrapped
  } else {
    gen_expression(env, key, context, None, None)?
  };
  let event_options = if modifiers.options.len() == 0 && !effect {
    Either4::C(None)
  } else {
    let mut result = vec![if effect {
      Either4::D(vec![Either3::C(Some("effect: true".to_string()))])
    } else {
      Either4::C(None)
    }];
    result.extend(
      modifiers
        .options
        .iter()
        .map(|option| Either4::D(vec![Either3::C(Some(format!("{option}: true")))])),
    );
    Either4::D(gen_multi(get_delimiters_object_newline(), result))
  };
  let handler = gen_event_handler(env, context, value, Some(modifiers), false)?;

  if delegate {
    // key is static
    let delegates = context.get_named_property::<Object>("delegates")?;
    delegates
      .get_named_property::<Function<String, Object>>("add")?
      .apply(delegates, key_content.clone())?;
    // if this is the only delegated event of this name on this element,
    // we can generate optimized handler attachment code
    // e.g. n1.$evtclick = () => {}
    if !context
      .get_named_property::<Object>("block")?
      .get_named_property::<Vec<Object>>("operation")?
      .iter()
      .any(|op| {
        op.get_named_property::<String>("type")
          .unwrap()
          .eq("SET_EVENT")
          && op
            .get_named_property::<Object>("key")
            .unwrap()
            .get_named_property::<Object>("ast")
            .unwrap()
            .get_named_property::<u32>("start")
            .unwrap()
            != oper_key_strat
          && op.get_named_property::<bool>("delegate").unwrap()
          && op.get_named_property::<i32>("element").unwrap() == oper.element
          && op
            .get_named_property::<Object>("key")
            .unwrap()
            .get_named_property::<String>("content")
            .unwrap()
            == key_content
      })
    {
      let mut result = vec![
        Either3::A(Newline),
        Either3::C(Some(format!("n{element}.$evt{} = ", key_content))),
      ];
      result.extend(handler);
      return Ok(result);
    }
  }

  let mut result = vec![Either3::A(Newline)];
  result.extend(gen_call(
    Either::A(
      context
        .get_named_property::<Function<String, String>>("helper")?
        .call(if delegate {
          "delegate".to_string()
        } else {
          "on".to_string()
        })?,
    ),
    vec![
      Either4::C(Some(format!("n{element}"))),
      Either4::D(name),
      Either4::D(handler),
      event_options,
    ],
  ));

  Ok(result)
}

#[napi]
pub fn gen_event_handler(
  env: Env,
  context: Object,
  value: Option<SimpleExpressionNode>,
  modifiers: Option<Modifiers>,
  // passed as component prop - need additional wrap
  extra_wrap: bool,
) -> Result<Vec<CodeFragment>> {
  let mut handler_exp = vec![Either3::C(Some("() => {}".to_string()))];
  if let Some(value) = value
    && !value.content.trim().is_empty()
  {
    // Determine how the handler should be wrapped so it always reference the
    // latest value when invoked.
    if is_member_expression(&value) {
      // e.g. @click="foo.bar"
      handler_exp = gen_expression(env, value, context, None, None)?;
      if !extra_wrap {
        // non constant, wrap with invocation as `e => foo.bar(e)`
        // when passing as component handler, access is always dynamic so we
        // can skip this
        handler_exp.insert(0, Either3::C(Some("e => ".to_string())));
        handler_exp.push(Either3::C(Some("(e)".to_string())))
      }
    } else if is_fn_expression(&value) {
      // Fn expression: @click="e => foo(e)"
      // no need to wrap in this case
      handler_exp = gen_expression(env, value, context, None, None)?
    } else {
      // inline statement
      let has_multiple_statements = value.content.contains(";");
      handler_exp = vec![
        Either3::C(Some("() => ".to_string())),
        Either3::C(Some(if has_multiple_statements {
          "{".to_string()
        } else {
          "(".to_string()
        })),
      ];
      handler_exp.extend(gen_expression(env, value, context, None, None)?);
      handler_exp.push(Either3::C(Some(if has_multiple_statements {
        "}".to_string()
      } else {
        ")".to_string()
      })));
    }
  }

  let Modifiers { keys, non_keys, .. } = modifiers.unwrap_or(Modifiers {
    options: vec![],
    keys: vec![],
    non_keys: vec![],
  });
  if non_keys.len() > 0 {
    handler_exp = gen_call(
      Either::A(
        context
          .get_named_property::<Function<String, String>>("helper")?
          .call("withModifiers".to_string())?,
      ),
      vec![
        Either4::D(handler_exp),
        Either4::C(Some(format!(
          "[{}]",
          non_keys
            .iter()
            .map(|key| format!("\"{}\"", key))
            .collect::<Vec<_>>()
            .join(",")
        ))),
      ],
    );
  }

  if keys.len() > 0 {
    handler_exp = gen_call(
      Either::A(
        context
          .get_named_property::<Function<String, String>>("helper")?
          .call("withKeys".to_string())?,
      ),
      vec![
        Either4::D(handler_exp),
        Either4::C(Some(format!(
          "[{}]",
          keys
            .iter()
            .map(|key| format!("\"{}\"", key))
            .collect::<Vec<_>>()
            .join(",")
        ))),
      ],
    )
  }

  if extra_wrap {
    handler_exp.insert(0, Either3::C(Some("() => ".to_string())));
  }
  Ok(handler_exp)
}

#[napi]
pub fn gen_set_dynamic_events(
  env: Env,
  oper: SetDynamicEventsIRNode,
  context: Object,
) -> Result<Vec<CodeFragment>> {
  let mut result = vec![Either3::A(Newline)];
  result.extend(gen_call(
    Either::A(
      context
        .get_named_property::<Function<String, String>>("helper")?
        .call("setDynamicEvents".to_string())?,
    ),
    vec![
      Either4::C(Some(format!("n{}", oper.element))),
      Either4::D(gen_expression(env, oper.value, context, None, None)?),
    ],
  ));
  Ok(result)
}
