use std::{collections::HashSet, sync::LazyLock};

use napi::{
  Either, Env, Result,
  bindgen_prelude::{Either18, JsObjectValue, Object},
};
use regex::Regex;

use crate::{
  ir::index::{IRNodeTypes, Modifiers, SetEventIRNode, SimpleExpressionNode},
  transform::{DirectiveTransformResult, is_operation, reference, register_effect},
  utils::{
    check::is_jsx_component,
    error::{ErrorCodes, on_error},
    expression::{EMPTY_EXPRESSION, create_simple_expression, resolve_expression},
    text::get_text,
  },
};

pub fn transform_v_on(
  env: Env,
  dir: Object,
  node: Object,
  context: Object,
) -> Result<Option<DirectiveTransformResult>> {
  let Ok(name) = dir.get_named_property::<Object>("name") else {
    return Ok(None);
  };
  let is_component = is_jsx_component(node);

  let regex = Regex::new(r"^on([A-Z])").unwrap();
  let replaced = regex.replace(&get_text(name, context), |caps: &regex::Captures| {
    format!("on{}", caps[1].to_lowercase())
  })[2..]
    .to_string();
  let splited: Vec<&str> = replaced.split("_").collect();
  let name_string = splited[0].to_string();
  let modifiers = splited[1..].to_vec();

  let value = dir.get_named_property::<Object>("value");
  if value.is_err() && modifiers.is_empty() {
    on_error(env, ErrorCodes::X_V_ON_NO_EXPRESSION, context);
  }

  let mut arg = create_simple_expression(name_string.clone(), Some(true), Some(name), None);
  let exp = if let Ok(value) = value {
    Some(resolve_expression(value, context))
  } else {
    None
  };

  let Modifiers {
    keys: key_modifiers,
    non_keys: non_key_modifiers,
    options: event_option_modifiers,
  } = resolve_modifiers(
    if arg.is_static {
      Either::B(format!("on{}", name_string))
    } else {
      Either::A(&arg)
    },
    modifiers
      .iter()
      .map(|modifier| create_simple_expression(modifier.to_string(), None, None, None))
      .collect(),
  );

  let mut key_override = None;
  let is_static_click = arg.is_static && arg.content.to_lowercase() == "click";

  // normalize click.right and click.middle since they don't actually fire
  if non_key_modifiers
    .iter()
    .any(|modifier| modifier == "middle")
  {
    if key_override.is_some() {
      // TODO error here
    }

    if is_static_click {
      arg.content = "mouseup".to_string()
    } else if !arg.is_static {
      key_override = Some(("click".to_string(), "mouseup".to_string()))
    }
  }
  if non_key_modifiers.iter().any(|modifier| modifier == "right") {
    if is_static_click {
      arg.content = "contextmenu".to_string();
    } else if !arg.is_static {
      key_override = Some(("click".to_string(), "contextmenu".to_string()))
    }
  }

  if is_component {
    return Ok(Some(DirectiveTransformResult {
      key: arg,
      value: if let Some(exp) = exp {
        exp
      } else {
        EMPTY_EXPRESSION
      },
      handler: Some(true),
      handler_modifiers: Some(Modifiers {
        keys: key_modifiers,
        non_keys: non_key_modifiers,
        options: event_option_modifiers,
      }),
      model: None,
      model_modifiers: None,
      modifier: None,
      runtime_camelize: None,
    }));
  }

  // Only delegate if:
  // - no dynamic event name
  // - no event option modifiers (passive, capture, once)
  // - is a delegatable
  let delegate = arg.is_static
    && event_option_modifiers.len() == 0
    && DELEGATED_EVENTS.contains(arg.content.as_str());

  register_effect(
    &context,
    is_operation(vec![&arg], &context),
    Either18::H(SetEventIRNode {
      _type: IRNodeTypes::SET_EVENT,
      element: reference(context)?,
      value: exp,
      modifiers: Modifiers {
        keys: key_modifiers,
        non_keys: non_key_modifiers,
        options: event_option_modifiers,
      },
      key_override,
      delegate,
      effect: !arg.is_static,
      key: arg,
    }),
    None,
    None,
  )?;
  Ok(None)
}

static DELEGATED_EVENTS: LazyLock<HashSet<&str>> = LazyLock::new(|| {
  HashSet::from([
    "beforeinput",
    "click",
    "dblclick",
    "contextmenu",
    "focusin",
    "focusout",
    "input",
    "keydown",
    "keyup",
    "mousedown",
    "mousemove",
    "mouseout",
    "mouseover",
    "mouseup",
    "pointerdown",
    "pointermove",
    "pointerout",
    "pointerover",
    "pointerup",
    "touchend",
    "touchmove",
    "touchstart",
  ])
});

fn is_event_option_modifier(modifier: &str) -> bool {
  matches!(modifier, "passive" | "once" | "capture")
}

fn is_non_key_modifier(modifier: &str) -> bool {
  matches!(
    modifier,
    // event propagation management
    "stop" | "prevent" | "self" |
    // system modifiers + exact
    "ctrl" | "shift" | "alt" | "meta" | "exact" |
    // mouse
    "middle"
  )
}

// left & right could be mouse or key modifiers based on event type
fn maybe_key_modifier(modifier: &str) -> bool {
  matches!(modifier, "left" | "right")
}

fn is_keyboard_event(key: &str) -> bool {
  matches!(key, "onkeyup" | "onkeydown" | "onkeypress")
}

pub fn resolve_modifiers(
  key: Either<&SimpleExpressionNode, String>,
  modifiers: Vec<SimpleExpressionNode>,
) -> Modifiers {
  let mut key_modifiers: Vec<String> = vec![];
  let mut non_key_modifiers: Vec<String> = vec![];
  let mut event_option_modifiers: Vec<String> = vec![];

  for modifier in modifiers {
    let modifier = modifier.content;
    if is_event_option_modifier(&modifier) {
      // eventOptionModifiers: modifiers for addEventListener() options,
      // e.g. .passive & .capture
      event_option_modifiers.push(modifier);
    } else {
      let key_string = match &key {
        Either::A(node) => {
          if node.is_static {
            &node.content
          } else {
            ""
          }
        }
        Either::B(string) => &string,
      };

      // runtimeModifiers: modifiers that needs runtime guards
      if maybe_key_modifier(&modifier) {
        if !key_string.is_empty() {
          if is_keyboard_event(&key_string.to_lowercase()) {
            key_modifiers.push(modifier);
          } else {
            non_key_modifiers.push(modifier)
          }
        } else {
          key_modifiers.push(modifier.clone());
          non_key_modifiers.push(modifier)
        }
      } else if is_non_key_modifier(&modifier) {
        non_key_modifiers.push(modifier)
      } else {
        key_modifiers.push(modifier)
      }
    }
  }

  Modifiers {
    keys: key_modifiers,
    non_keys: non_key_modifiers,
    options: event_option_modifiers,
  }
}
