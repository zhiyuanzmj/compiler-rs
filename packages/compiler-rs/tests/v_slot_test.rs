use compiler_rs::compile::compile;
use insta::assert_snapshot;

#[test]
fn v_slot_test() {
  let result = compile("<Comp v-slot:named={{ foo }}>{{ foo: foo }}</Comp>", None);
  assert_snapshot!(result.code);
}
