use std::sync::LazyLock;

use napi::{Either, bindgen_prelude::Either3};
use napi_derive::napi;
use regex::Regex;

use crate::ir::index::SourceLocation;

#[napi]
pub enum NewlineType {
  /** Start with `\n` */
  Start = 0,
  /** Ends with `\n` */
  End = -1,
  /** No `\n` included */
  None = -2,
  /** Don't know, calc it */
  Unknown = -3,
}

#[napi]
pub type Fragment = (
  String,
  Option<NewlineType>,
  Option<SourceLocation>,
  Option<String>,
);

#[napi]
pub enum FragmentSymbol {
  Newline,
  IndentStart,
  IndentEnd,
}

pub enum CodeFragment {
  Newline(FragmentSymbol),
  IndentStart(FragmentSymbol),
  IndentEnd(FragmentSymbol),
  String(String),
  Fragment(Fragment),
}

static VALID_ASSET_REGEX: LazyLock<regex::Regex> =
  LazyLock::new(|| regex::Regex::new(r"[^A-Za-z0-9_$]").unwrap());

#[napi]
pub fn to_valid_asset_id(name: String, _type: String) -> String {
  let name = VALID_ASSET_REGEX
    .replace_all(name.as_str(), |caps: &regex::Captures| {
      let ch = caps.get(0).unwrap().as_str();
      if ch == "-" {
        "_".to_string()
      } else {
        (ch.chars().next().unwrap() as u32).to_string()
      }
    })
    .to_string();

  format!("_{_type}_{name}")
}
