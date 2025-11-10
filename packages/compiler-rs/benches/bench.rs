use compiler_rs::compile::compile;
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_compile(b: &mut Criterion) {
  b.bench_function("compile", |b| {
    b.iter(|| {
      compile(
        format!(
          "<>{}</>",
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
            .repeat(13)
        ),
        None,
      )
    })
  });
}

criterion_group!(benches, bench_compile);
criterion_main!(benches);
