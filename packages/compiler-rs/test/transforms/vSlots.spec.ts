import { compile } from '@vue-jsx-vapor/compiler-rs'
import { describe, expect, test } from 'vitest'

describe('compiler: transform v-slots', () => {
  test('basic', () => {
    const { code } = compile(`<Comp v-slots={{ default: ({ foo })=> <>{ foo + bar }</> }}> </Comp>`)
    expect(code).toMatchSnapshot()
  })
})
