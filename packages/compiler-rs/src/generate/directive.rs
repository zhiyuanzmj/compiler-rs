use napi::Either;
use napi::Env;
use napi::Result;
use napi::bindgen_prelude::Either3;
use napi::bindgen_prelude::Either4;
use napi::bindgen_prelude::Function;
use napi::bindgen_prelude::JsObjectValue;
use napi::bindgen_prelude::Object;
use napi_derive::napi;

use crate::generate::expression::gen_expression;
use crate::generate::utils::CodeFragment;
use crate::generate::utils::FragmentSymbol::Newline;
use crate::generate::utils::gen_call;
use crate::generate::utils::gen_multi;
use crate::generate::utils::get_delimiters_array;
use crate::generate::utils::to_valid_asset_id;
use crate::generate::v_model::gen_v_model;
use crate::generate::v_show::gen_v_show;
use crate::ir::index::DirectiveIRNode;
use crate::ir::index::DirectiveNode;
use crate::ir::index::IRNodeTypes;
use crate::utils::check::is_simple_identifier;
use crate::utils::expression::create_simple_expression;

#[napi]
pub fn gen_builtin_directive(
  env: Env,
  oper: DirectiveIRNode,
  context: Object<'static>,
) -> Result<Vec<CodeFragment>> {
  match oper.name.as_str() {
    "show" => gen_v_show(env, oper, context),
    "model" => gen_v_model(env, oper, context),
    _ => Ok(vec![]),
  }
}

#[napi]
/**
 * user directives via `withVaporDirectives`
 * TODO the compiler side is implemented but no runtime support yet
 * it was removed due to perf issues
 */
pub fn gen_directives_for_element(env: Env, id: i32, context: Object) -> Result<Vec<CodeFragment>> {
  let dirs = context
    .get_named_property::<Object>("block")?
    .get_named_property::<Vec<Object>>("operation")?
    .into_iter()
    .filter(|oper| {
      oper.get_named_property::<IRNodeTypes>("type").unwrap() == IRNodeTypes::DIRECTIVE
        && oper.get_named_property::<i32>("element").unwrap() == id
        && !oper.get_named_property::<bool>("builtin").unwrap_or(false)
    })
    .collect::<Vec<_>>();
  if dirs.len() > 0 {
    let element = format!("n{}", dirs[0].get_named_property::<i32>("element")?);
    let directive_items = dirs
      .into_iter()
      .map(move |item| {
        let dir = item.get_named_property::<DirectiveNode>("dir").unwrap();
        let name = item.get_named_property::<String>("name").unwrap();
        let asset = item.get_named_property::<bool>("asset").ok();
        let directive_var = if asset.unwrap_or(false) {
          Either4::C(Some(to_valid_asset_id(name, "directive".to_string())))
        } else {
          Either4::D(
            gen_expression(
              env,
              create_simple_expression(name, None, None, None),
              context,
              None,
              None,
            )
            .unwrap(),
          )
        };
        let value = if let Some(exp) = dir.exp {
          let mut result = gen_expression(env, exp, context, None, None).unwrap();
          result.insert(0, Either3::C(Some("() => ".to_string())));
          Either4::D(result)
        } else {
          Either4::C(None)
        };
        let argument = if let Some(arg) = dir.arg {
          Either4::D(gen_expression(env, arg, context, None, None).unwrap())
        } else {
          Either4::C(None)
        };
        let modifiers = if &dir.modifiers.len() > &0 {
          Either4::D(vec![
            Either3::C(Some("{ ".to_string())),
            Either3::C(Some(gen_directive_modifiers(
              dir.modifiers.into_iter().map(|m| m.content).collect(),
            ))),
            Either3::C(Some(" }".to_string())),
          ])
        } else {
          Either4::C(None)
        };

        Either4::D(gen_multi(
          (
            Either4::C(Some("[".to_string())),
            Either4::C(Some("]".to_string())),
            Either4::C(Some(", ".to_string())),
            Some("void 0".to_string()),
          ),
          vec![directive_var, value, argument, modifiers],
        ))
      })
      .collect::<_>();
    let directives = gen_multi(get_delimiters_array(), directive_items);
    let mut result = vec![Either3::A(Newline)];
    result.extend(gen_call(
      Either::A(
        context
          .get_named_property::<Function<String, String>>("helper")?
          .call("withVaporDirectives".to_string())?,
      ),
      vec![Either4::C(Some(element)), Either4::D(directives)],
    ));
    Ok(result)
  } else {
    Ok(vec![])
  }
}

#[napi]
pub fn gen_directive_modifiers(modifiers: Vec<String>) -> String {
  modifiers
    .into_iter()
    .map(|value| {
      format!(
        "{}: true",
        if is_simple_identifier(&value) {
          value
        } else {
          format!("\"{value}\"")
        }
      )
    })
    .collect::<Vec<_>>()
    .join(", ")
}
