/* eslint-disable */
import { compile as jsCompile } from '@vue-jsx-vapor/compiler'
import { compile as rsCompile } from '@vue-jsx-vapor/compiler-rs'
import { Bench } from 'tinybench'
import { transformSync } from '@babel/core'
import vueJsx from '@vue/babel-plugin-jsx'
function vueJsxCompile(source) {
  transformSync(source, {
    plugins: [vueJsx],
    filename: 'index.jsx',
    sourceMaps: false,
    sourceFileName: 'index.jsx',
    babelrc: false,
    configFile: false,
  })
}

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
source = `<>${source.repeat(12)}</>`

console.time('@vue-jsx-vapor/compiler-rs + oxc-parser    ')
rsCompile(source)
console.timeEnd('@vue-jsx-vapor/compiler-rs + oxc-parser    ')

console.time('@vue-jsx-vapor/compiler    + babel-parser  ')
jsCompile(source)
console.timeEnd('@vue-jsx-vapor/compiler    + babel-parser  ')

console.time('vue-jsx                    + babel-parser  ')
vueJsxCompile(source)
console.timeEnd('vue-jsx                    + babel-parser  ')

bench.add('compiler-rs + oxc-parser', () => {
  rsCompile(source, {})
})

bench.add('compiler-js + babel-parser', () => {
  jsCompile(source)
})

bench.add('vue-jsx + babel-parser', () => {
  vueJsxCompile(source)
})

await bench.run()

console.table(bench.table())
