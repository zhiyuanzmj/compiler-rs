import { describe, expect, test } from 'vitest'
import { makeCompile } from './_utils'

const compileWithSlots = makeCompile()

describe('compiler: transform v-slots', () => {
  test('basic', () => {
    const { code } = compileWithSlots(
      `<Comp v-slots={{ default: ({ foo })=> <>{ foo + bar }</> }}> </Comp>`,
    )
    expect(code).toMatchSnapshot()
  })
})
