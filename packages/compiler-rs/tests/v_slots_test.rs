use compiler_rs::compile::compile;
use insta::assert_snapshot;

#[test]
fn v_slots_test() {
  let result = compile(
    "<Comp v-slot={{ bar }}>
      <Comp bar={bar} v-slots={{ bar, default: ({ foo })=> <>{ foo + bar } {<Comp v-slot={{baz}}>{bar}{baz}</Comp>}</> }}>
      </Comp>{bar}
    </Comp>",
    None,
  );
  // assert_snapshot!(format!("{:?}", result.templates));
  assert_snapshot!(result.code);
}
