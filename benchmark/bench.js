/* eslint-disable */
import { compile as jsCompile } from '@vue-jsx-vapor/compiler'
import { compile as oxcCompile } from '@vue-jsx-vapor/compiler-oxc'
import { compile as rsCompile } from '@vue-jsx-vapor/compiler-rs'
import { Bench } from 'tinybench'

const bench = new Bench()

let source = `
<Comp
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
</Comp>`
source = `<>${source.repeat(13)}</>`

console.time('compiler.rs + oxc-parser')
rsCompile(source)
console.timeEnd('compiler.rs + oxc-parser')

console.time('compiler.js + oxc-parser')
oxcCompile(source)
console.timeEnd('compiler.js + oxc-parser')

console.time('compiler.js + babel-parser')
jsCompile(source)
console.timeEnd('compiler.js + babel-parser')

bench.add('compiler.rs + oxc-parser', () => {
  rsCompile(source, {})
})

bench.add('compiler.js + oxc-parser', () => {
  oxcCompile(source)
})

bench.add('compiler.js + babel-parser', () => {
  jsCompile(source)
})

await bench.run()

console.table(bench.table())
