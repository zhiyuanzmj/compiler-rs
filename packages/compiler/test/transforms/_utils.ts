import { generate, transform, type CompilerOptions } from '../../src'
import { IRNodeTypes } from '../../src/ir'
import { parseExpression } from '../../src/utils'

export function makeCompile(options: CompilerOptions = {}) {
  return (source: string, overrideOptions: CompilerOptions = {}) => {
    const expression = parseExpression('index.tsx', source)
    const children =
      expression.type === 'JSXFragment'
        ? expression.children
        : expression.type === 'JSXElement'
          ? [expression]
          : []
    const ast = {
      type: IRNodeTypes.ROOT,
      children,
    } as any
    const ir = transform(ast, {
      filename: 'index.tsx',
      source,
      ...options,
      ...overrideOptions,
    }) as any
    return {
      ir,
      ...generate(ir, {
        ...options,
        ...overrideOptions,
      }),
    }
  }
}
