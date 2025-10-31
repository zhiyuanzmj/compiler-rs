import { compile as jsCompile } from '@vue-jsx-vapor/compiler'
import { compile as rsCompile } from '@vue-jsx-vapor/compiler-napi'
import { compile as oxcCompile } from '@vue-jsx-vapor/compiler-oxc'
import { Bench } from 'tinybench'

const bench = new Bench()

let source = `
<Comp v-if={foo} onSubmit={submit}>
  <div v-for={i in list} key={i} id={i}>
    <Bar v-model={foo}>
      {i}
      <template v-slot:name={foo}>
        {foo}
      </template>
    </Bar>
  </div>
</Comp>
`

const options = {
  filename: 'index.tsx',
  source,
  templates: [],
  withFallback: false,
  isTS: true,
  sourceMap: false,
  isCustomElement: () => false,
  onError: (e) => {
    throw e
  },
}

bench.add('compiler-rs + oxc-parser', () => {
  rsCompile(source, options)
})

bench.add('compiler-js + oxc-parser', () => {
  oxcCompile(source, options)
})

bench.add('compiler-js + babel-parser', () => {
  jsCompile(source, options)
})

await bench.run()

console.table(bench.table())
