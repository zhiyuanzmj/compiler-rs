import { compile as jsCompile } from '@vue-jsx-vapor/compiler'
import * as nativeCompile from '@vue-jsx-vapor/compiler-oxc'
import { Bench } from 'tinybench'

const bench = new Bench()

const source = `<Comp v-test>
<div v-if="true">
  <Bar v-hello_world />
</div>
</Comp>`

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

bench.add('Native', () => {
  nativeCompile.compile(source, options)
})

bench.add('JavaScript', () => {
  jsCompile(source, options)
})

await bench.run()

console.table(bench.table())
