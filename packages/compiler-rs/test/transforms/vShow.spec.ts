import { compile, ErrorCodes } from '@vue-jsx-vapor/compiler-rs'
import { describe, expect, test, vi } from 'vitest'

describe('compiler: v-show transform', () => {
  test('simple expression', () => {
    const { code } = compile(`<div v-show={foo}/>`)
    expect(code).toMatchSnapshot()
  })

  test('should raise error if has no expression', () => {
    const onError = vi.fn()
    compile(`<div v-show/>`, { onError })

    expect(onError).toHaveBeenCalledTimes(1)
    expect(onError).toHaveBeenCalledWith(
      expect.objectContaining({
        code: ErrorCodes.VShowNoExpression,
      }),
    )
  })
})
