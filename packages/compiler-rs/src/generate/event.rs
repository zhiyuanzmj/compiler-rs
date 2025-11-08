use napi::Either;
use napi::bindgen_prelude::Either3;
use napi::bindgen_prelude::Either4;

use crate::generate::CodegenContext;
use crate::generate::expression::gen_expression;
use crate::generate::utils::CodeFragment;
use crate::generate::utils::FragmentSymbol::Newline;
use crate::generate::utils::gen_call;
use crate::generate::utils::gen_multi;
use crate::generate::utils::get_delimiters_object_newline;
use crate::ir::index::Modifiers;
use crate::ir::index::SetDynamicEventsIRNode;
use crate::ir::index::SetEventIRNode;
use crate::ir::index::SimpleExpressionNode;
use crate::utils::check::is_member_expression;

pub fn gen_set_event(
  oper: SetEventIRNode,
  context: &CodegenContext,
  event_opers: &Vec<SetEventIRNode>,
) -> Vec<CodeFragment> {
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
  let oper_key_strat = key.loc.as_ref().unwrap().start;
  let name = if let Some(key_override) = key_override {
    let find = format!("\"{}\"", key_override.0);
    let replacement = format!("\"{}\"", key_override.1);
    let mut wrapped = vec![Either3::C(Some("(".to_string()))];
    wrapped.extend(gen_expression(key, context, None, None));
    wrapped.push(Either3::C(Some(")".to_string())));
    let cloned = wrapped.clone();
    wrapped.push(Either3::C(Some(format!(" === {find} ? {replacement} : "))));
    wrapped.extend(cloned);
    wrapped
  } else {
    gen_expression(key, context, None, None)
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
  let handler = gen_event_handler(context, value, Some(modifiers), false);

  if delegate {
    // key is static
    context.delegates.borrow_mut().insert(key_content.clone());
    // if this is the only delegated event of this name on this element,
    // we can generate optimized handler attachment code
    // e.g. n1.$evtclick = () => {}
    if !event_opers.iter().any(|op| {
      if op.key.loc.as_ref().unwrap().start != oper_key_strat
        && op.delegate
        && op.element == oper.element
        && op.key.content == key_content
      {
        true
      } else {
        false
      }
    }) {
      let mut result = vec![
        Either3::A(Newline),
        Either3::C(Some(format!("n{element}.$evt{} = ", key_content))),
      ];
      result.extend(handler);
      return result;
    }
  }

  let mut result = vec![Either3::A(Newline)];
  result.extend(gen_call(
    Either::A(context.helper(if delegate { "delegate" } else { "on" })),
    vec![
      Either4::C(Some(format!("n{element}"))),
      Either4::D(name),
      Either4::D(handler),
      event_options,
    ],
  ));
  result
}

pub fn gen_event_handler(
  context: &CodegenContext,
  value: Option<SimpleExpressionNode>,
  modifiers: Option<Modifiers>,
  // passed as component prop - need additional wrap
  extra_wrap: bool,
) -> Vec<CodeFragment> {
  let mut handler_exp = vec![Either3::C(Some("() => {}".to_string()))];
  if let Some(value) = value
    && !value.content.trim().is_empty()
  {
    // Determine how the handler should be wrapped so it always reference the
    // latest value when invoked.
    if is_member_expression(&value) {
      // e.g. @click="foo.bar"
      handler_exp = gen_expression(value, context, None, None);
      if !extra_wrap {
        // non constant, wrap with invocation as `e => foo.bar(e)`
        // when passing as component handler, access is always dynamic so we
        // can skip this
        handler_exp.insert(0, Either3::C(Some("e => ".to_string())));
        handler_exp.push(Either3::C(Some("(e)".to_string())))
      }
    } else if value.ast.as_ref().unwrap().is_function() {
      // Fn expression: @click="e => foo(e)"
      // no need to wrap in this case
      handler_exp = gen_expression(value, context, None, None)
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
      handler_exp.extend(gen_expression(value, context, None, None));
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
      Either::A(context.helper("withModifiers")),
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
      Either::A(context.helper("withKeys")),
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
  handler_exp
}

pub fn gen_set_dynamic_events(
  oper: SetDynamicEventsIRNode,
  context: &CodegenContext,
) -> Vec<CodeFragment> {
  let mut result = vec![Either3::A(Newline)];
  result.extend(gen_call(
    Either::A(context.helper("setDynamicEvents")),
    vec![
      Either4::C(Some(format!("n{}", oper.element))),
      Either4::D(gen_expression(oper.value, context, None, None)),
    ],
  ));
  result
}
