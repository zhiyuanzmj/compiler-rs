use compiler_rs::compile::compile;

#[test]
pub fn test_compile() {
  compile(
    "<Comp
    v-if={true}
    foo={foo}
    ref={foo}
    onClick={()=> alert(1)}
    v-show={true}
    v-model={foo}
    v-test
    v-slot={foo}
  >
    <div v-for={({item}, index) in list} key={key} v-once>
      {item}
    </div>
    <Foo v-if={foo}>
      default
      <template v-slot:bar={{ bar }}>
        {bar}
      </template>
    </Foo>
  </Comp>"
      .to_string(),
    None,
  );
}
