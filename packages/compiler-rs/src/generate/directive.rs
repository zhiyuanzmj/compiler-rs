use napi::Either;
use napi::Result;
use napi::bindgen_prelude::Either3;
use napi::bindgen_prelude::Either4;
use napi::bindgen_prelude::Either16;
use napi_derive::napi;

use crate::generate::CodegenContext;
use crate::generate::expression::gen_expression;
use crate::generate::utils::CodeFragment;
use crate::generate::utils::CodeFragments;
use crate::generate::utils::FragmentSymbol::Newline;
use crate::generate::utils::gen_call;
use crate::generate::utils::gen_multi;
use crate::generate::utils::get_delimiters_array;
use crate::generate::utils::to_valid_asset_id;
use crate::generate::v_model::gen_v_model;
use crate::generate::v_show::gen_v_show;
use crate::ir::index::BlockIRNode;
use crate::ir::index::DirectiveIRNode;
use crate::utils::check::is_simple_identifier;
use crate::utils::expression::create_simple_expression;

pub fn gen_builtin_directive(
  oper: DirectiveIRNode,
  context: &CodegenContext,
) -> Result<Vec<CodeFragment>> {
  match oper.name.as_str() {
    "show" => gen_v_show(oper, context),
    "model" => gen_v_model(oper, context),
    _ => Ok(vec![]),
  }
}

/**
 * user directives via `withVaporDirectives`
 * TODO the compiler side is implemented but no runtime support yet
 * it was removed due to perf issues
 */
pub fn gen_directives_for_element(
  id: i32,
  context: &CodegenContext,
  context_block: &mut BlockIRNode,
) -> Result<Vec<CodeFragment>> {
  let mut element = String::new();
  let mut directive_items: Vec<CodeFragments> = vec![];
  for item in &mut context_block.operation {
    if let Either16::M(item) = item
      && item.element == id
      && !item.builtin.unwrap_or(false)
    {
      if element.is_empty() {
        element = item.element.to_string();
      }
      let name = item.name.clone();
      let asset = item.asset;
      let directive_var = if asset.unwrap_or(false) {
        Either4::C(Some(to_valid_asset_id(name, "directive".to_string())))
      } else {
        Either4::D(
          gen_expression(
            create_simple_expression(name, None, None, None),
            context,
            None,
            None,
          )
          .unwrap(),
        )
      };
      let value = if let Some(ref exp) = item.dir.exp {
        let mut result = gen_expression(exp.clone(), context, None, None).unwrap();
        result.insert(0, Either3::C(Some("() => ".to_string())));
        Either4::D(result)
      } else {
        Either4::C(None)
      };
      let argument = if let Some(ref arg) = item.dir.arg {
        Either4::D(gen_expression(arg.clone(), context, None, None).unwrap())
      } else {
        Either4::C(None)
      };
      let modifiers = if &item.dir.modifiers.len() > &0 {
        Either4::D(vec![
          Either3::C(Some("{ ".to_string())),
          Either3::C(Some(gen_directive_modifiers(
            item
              .dir
              .modifiers
              .iter()
              .map(|m| m.content.clone())
              .collect(),
          ))),
          Either3::C(Some(" }".to_string())),
        ])
      } else {
        Either4::C(None)
      };

      directive_items.push(Either4::D(gen_multi(
        (
          Either4::C(Some("[".to_string())),
          Either4::C(Some("]".to_string())),
          Either4::C(Some(", ".to_string())),
          Some("void 0".to_string()),
        ),
        vec![directive_var, value, argument, modifiers],
      )));
    }
  }
  if directive_items.len() == 0 {
    return Ok(vec![]);
  }
  let directives = gen_multi(get_delimiters_array(), directive_items);
  let mut result = vec![Either3::A(Newline)];
  result.extend(gen_call(
    Either::A(context.helper("withVaporDirectives")),
    vec![
      Either4::C(Some(format!("n{}", element))),
      Either4::D(directives),
    ],
  ));
  Ok(result)
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
