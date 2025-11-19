import { compile } from '@vue-jsx-vapor/compiler-rs'
import { describe, expect, test } from 'vitest'

describe('directive', () => {
  describe('custom directive', () => {
    test('basic', () => {
      const { code } = compile(`<div v-example></div>`, { withFallback: true })
      expect(code).matchSnapshot()
    })

    test('binding value', () => {
      const { code } = compile(`<div v-example={msg}></div>`)
      expect(code).matchSnapshot()
    })

    test('static parameters', () => {
      const { code } = compile(`<div v-example:foo={msg}></div>`)
      expect(code).matchSnapshot()
    })

    test('modifiers', () => {
      const { code } = compile(`<div v-example_bar={msg}></div>`)
      expect(code).matchSnapshot()
    })

    test('modifiers w/o binding', () => {
      const { code } = compile(`<div v-example_foo-bar></div>`)
      expect(code).matchSnapshot()
    })

    test('static parameters and modifiers', () => {
      const { code } = compile(`<div v-example:foo_bar={msg}></div>`)
      expect(code).matchSnapshot()
    })

    test('dynamic parameters', () => {
      const { code } = compile(`<div v-example:$foo$={msg}></div>`)
      expect(code).matchSnapshot()
    })

    test('component', () => {
      const { code } = compile(`
      <Comp v-test>
        <div v-if="true">
          <Bar v-hello_world />
        </div>
      </Comp>
      `)
      expect(code).matchSnapshot()
    })
  })
})
