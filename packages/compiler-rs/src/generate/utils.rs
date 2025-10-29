use std::sync::LazyLock;

use napi::{
  Either,
  bindgen_prelude::{Either3, Either4},
};
use napi_derive::napi;

use crate::ir::index::SourceLocation;

#[napi]
#[derive(Clone)]
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
pub type Fragment = (String, NewlineType, Option<SourceLocation>, Option<String>);

#[napi]
#[derive(Clone)]
pub enum FragmentSymbol {
  Newline = 1,
  IndentStart = 2,
  IndentEnd = 3,
}

#[napi]
pub type CodeFragment = Either3<FragmentSymbol, Fragment, Option<String>>;

#[napi]
pub type CodeFragments = Either4<FragmentSymbol, Fragment, Option<String>, Vec<CodeFragment>>;

#[napi]
pub type CodeFragmentDelimiters = (CodeFragments, CodeFragments, CodeFragments, Option<String>);

#[napi]
pub fn get_delimiters_array() -> CodeFragmentDelimiters {
  (
    Either4::C(Some(String::from("["))),
    Either4::C(Some(String::from("]"))),
    Either4::C(Some(String::from(", "))),
    None,
  )
}

#[napi]
pub fn get_delimiters_array_newline() -> CodeFragmentDelimiters {
  (
    Either4::D(vec![
      Either3::C(Some(String::from("["))),
      Either3::A(FragmentSymbol::IndentStart),
      Either3::A(FragmentSymbol::Newline),
    ]),
    Either4::D(vec![
      Either3::A(FragmentSymbol::IndentEnd),
      Either3::A(FragmentSymbol::Newline),
      Either3::C(Some(String::from("]"))),
    ]),
    Either4::D(vec![
      Either3::C(Some(String::from(","))),
      Either3::A(FragmentSymbol::Newline),
    ]),
    None,
  )
}

#[napi]
pub fn get_delimiters_object() -> CodeFragmentDelimiters {
  (
    Either4::C(Some(String::from("{ "))),
    Either4::C(Some(String::from(" }"))),
    Either4::C(Some(String::from(", "))),
    None,
  )
}

#[napi]
pub fn get_delimiters_object_newline() -> CodeFragmentDelimiters {
  (
    Either4::D(vec![
      Either3::C(Some(String::from("{"))),
      Either3::A(FragmentSymbol::IndentStart),
      Either3::A(FragmentSymbol::Newline),
    ]),
    Either4::D(vec![
      Either3::A(FragmentSymbol::IndentEnd),
      Either3::A(FragmentSymbol::Newline),
      Either3::C(Some(String::from("}"))),
    ]),
    Either4::D(vec![
      Either3::C(Some(String::from(","))),
      Either3::A(FragmentSymbol::Newline),
    ]),
    None,
  )
}

#[napi]
pub fn gen_multi(
  (left, right, seg, placeholder): CodeFragmentDelimiters,
  mut frags: Vec<CodeFragments>,
) -> Vec<CodeFragment> {
  if placeholder.is_some() {
    while frags.len() > 0 && matches!(frags.get(frags.len() - 1).unwrap(), Either4::C(None)) {
      frags.pop();
    }
    frags = frags
      .into_iter()
      .map(|frag| {
        if let Either4::C(ref frag) = frag
          && frag.is_none()
        {
          Either4::C(placeholder.clone())
        } else {
          frag
        }
      })
      .collect();
  } else {
    frags = frags
      .into_iter()
      .filter(|frag| {
        if let Either4::C(frag) = frag
          && frag.is_none()
        {
          false
        } else {
          true
        }
      })
      .collect()
  }

  let mut frag: Vec<CodeFragment> = vec![];
  let _frag = &mut frag;
  let mut push = move |item: CodeFragments| match item {
    Either4::A(item) => _frag.push(Either3::A(item)),
    Either4::B(item) => _frag.push(Either3::B(item)),
    Either4::C(item) => _frag.push(Either3::C(item)),
    Either4::D(item) => _frag.extend(item),
  };
  push(left);
  let mut i = 0;
  let len = frags.len();
  let seg = seg;
  for item in frags {
    push(item);
    if i < len - 1 {
      push(seg.clone())
    }
    i += 1;
  }
  push(right);
  frag
}

#[napi]
pub fn gen_call(
  node: Either<String, (String, Option<CodeFragment>)>,
  frags: Vec<CodeFragments>,
) -> Vec<CodeFragment> {
  let (fn_name, placeholder) = match node {
    Either::A(fn_name) => (fn_name, Some(String::from("null"))),
    Either::B(frag) => (
      frag.0,
      if let Some(Either3::C(placeholder)) = frag.1 {
        placeholder
      } else {
        None
      },
    ),
  };
  let mut result = vec![Either3::C(Some(fn_name))];
  result.extend(gen_multi(
    (
      Either4::C(Some("(".to_string())),
      Either4::C(Some(")".to_string())),
      Either4::C(Some(", ".to_string())),
      placeholder,
    ),
    frags,
  ));
  result
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
