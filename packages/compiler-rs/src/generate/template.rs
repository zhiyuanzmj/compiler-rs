use std::cell::RefCell;
use std::mem;
use std::rc::Rc;

use napi::Either;
use napi::Result;
use napi::bindgen_prelude::Either3;
use napi::bindgen_prelude::Either4;

use crate::generate::CodegenContext;
use crate::generate::directive::gen_directives_for_element;
use crate::generate::operation::gen_operation_with_insertion_state;
use crate::generate::utils::CodeFragment;
use crate::generate::utils::FragmentSymbol::Newline;
use crate::generate::utils::gen_call;
use crate::ir::index::BlockIRNode;
use crate::ir::index::DynamicFlag;
use crate::ir::index::IRDynamicInfo;
use crate::utils::my_box::MyBox;

pub fn gen_templates(
  templates: Vec<String>,
  root_index: Option<i32>,
  context: &CodegenContext,
) -> Result<Vec<String>> {
  let mut i = 0;
  Ok(
    templates
      .into_iter()
      .map(|template| {
        let result = if template.starts_with("_template") {
          template
        } else {
          format!(
            "{}(\"{}\"{})",
            context.helper("template"),
            template,
            if let Some(root_index) = root_index
              && i == root_index
            {
              ", true"
            } else {
              ""
            }
          )
        };
        i += 1;
        result
      })
      .collect(),
  )
}

pub fn gen_self(
  dynamic: IRDynamicInfo,
  context: &CodegenContext,
  context_block: &mut BlockIRNode,
) -> Result<Vec<CodeFragment>> {
  let mut frag = vec![];
  let IRDynamicInfo {
    id,
    children,
    template,
    operation,
    ..
  } = dynamic;

  if let Some(id) = id
    && let Some(template) = template
  {
    frag.push(Either3::A(Newline));
    frag.push(Either3::C(Some(format!("const n{id} = t{template}()"))));
    frag.extend(gen_directives_for_element(id, context, context_block)?);
  }

  if let Some(MyBox(operation)) = operation {
    frag.extend(gen_operation_with_insertion_state(
      *operation,
      context,
      context_block,
      &vec![],
    )?)
  }

  let result = {
    let _frag = &mut frag;
    gen_children(
      children,
      context,
      context_block,
      Rc::new(RefCell::new(move |value| _frag.extend(value))),
      format!("n{}", id.unwrap_or(0)),
    )?
  };
  frag.extend(result);

  Ok(frag)
}

fn gen_children(
  children: Vec<IRDynamicInfo>,
  context: &CodegenContext,
  context_block: &mut BlockIRNode,
  push_block: Rc<RefCell<impl FnMut(Vec<CodeFragment>)>>,
  from: String,
) -> Result<Vec<CodeFragment>> {
  let mut frag = vec![];

  let mut offset = 0;
  let mut prev: Option<(String, i32)> = None;

  let mut index = 0;
  for mut child in children {
    let mut _push_block = push_block.borrow_mut();
    if child.flags & DynamicFlag::NON_TEMPLATE as i32 != 0 {
      offset -= 1;
    }

    let id = if child.flags & DynamicFlag::REFERENCED as i32 != 0 {
      if child.flags & DynamicFlag::INSERT as i32 != 0 {
        child.anchor
      } else {
        child.id.clone()
      }
    } else {
      None
    };

    if id.is_none() && !child.has_dynamic_child.unwrap_or(false) {
      frag.extend(gen_self(child, context, context_block)?);
      index += 1;
      continue;
    }

    let element_index = index + offset;
    // p for "placeholder" variables that are meant for possible reuse by
    // other access paths
    let variable = if let Some(id) = id {
      format!("n{id}")
    } else {
      let temp_id = context_block.temp_id;
      context_block.temp_id = temp_id + 1;
      format!("p{}", temp_id)
    };
    _push_block(vec![
      Either3::A(Newline),
      Either3::C(Some(format!("const {variable} = "))),
    ]);

    if let Some(prev) = prev {
      if element_index - prev.1 == 1 {
        _push_block(gen_call(
          Either::A(context.helper("next")),
          vec![Either4::C(Some(prev.0))],
        ))
      } else {
        _push_block(gen_call(
          Either::A(context.helper("nthChild")),
          vec![
            Either4::C(Some(from.clone())),
            Either4::C(Some(element_index.to_string())),
          ],
        ))
      }
    } else if element_index == 0 {
      _push_block(gen_call(
        Either::A(context.helper("child")),
        vec![Either4::C(Some(from.clone()))],
      ))
    } else {
      // check if there's a node that we can reuse from
      let mut init = gen_call(
        Either::A(context.helper("child")),
        vec![Either4::C(Some(from.clone()))],
      );
      if element_index == 1 {
        init = gen_call(Either::A(context.helper("next")), vec![Either4::D(init)])
      } else if element_index > 1 {
        init = gen_call(
          Either::A(context.helper("nthChild")),
          vec![
            Either4::C(Some(from.clone())),
            Either4::C(Some(element_index.to_string())),
          ],
        )
      }
      _push_block(init)
    }

    let child_children = mem::take(&mut child.children);
    if id.eq(&child.anchor) && !child.has_dynamic_child.unwrap_or(false) {
      frag.extend(gen_self(child, context, context_block)?);
    }

    if let Some(id) = id {
      frag.extend(gen_directives_for_element(id, context, context_block)?);
    }

    prev = Some((variable.clone(), element_index));
    drop(_push_block);
    frag.extend(gen_children(
      child_children,
      context,
      context_block,
      Rc::clone(&push_block),
      variable,
    )?);

    index += 1;
  }
  Ok(frag)
}
