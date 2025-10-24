import { FragmentSymbol, NewlineType } from '@vue-jsx-vapor/compiler-rs'
import { isArray, isString } from '@vue/shared'
import { SourceMapGenerator } from 'source-map-js'
import type { CodegenContext } from '../generate'
import type { SourceLocation } from '../ir'
import { locStub } from './expression'

export { toValidAssetId } from '@vue-jsx-vapor/compiler-rs'

export { FragmentSymbol, NewlineType }

export const NEWLINE = 1 as const
export const INDENT_START = 2 as const
export const INDENT_END = 3 as const

interface CodegenSourceMapGenerator {
  setSourceContent: (sourceFile: string, sourceContent: string) => void
  toJSON: () => RawSourceMap
  _sources: Set<string>
  _names: Set<string>
  _mappings: {
    add: (mapping: MappingItem) => void
  }
}

interface RawSourceMap {
  file?: string
  sourceRoot?: string
  version: string
  sources: string[]
  names: string[]
  sourcesContent?: string[]
  mappings: string
}

interface MappingItem {
  source: string
  generatedLine: number
  generatedColumn: number
  originalLine: number
  originalColumn: number
  name: string | null
}

interface Position {
  line: number
  column: number
  index: number
}

type FalsyValue = false | null | undefined
export type CodeFragment =
  | typeof NEWLINE
  | typeof INDENT_START
  | typeof INDENT_END
  | string
  | [
      code: string,
      newlineIndex?: number,
      loc?: SourceLocation | null,
      name?: string,
    ]
  | FalsyValue
export type CodeFragments = Exclude<CodeFragment, any[]> | CodeFragment[]

export function buildCodeFragment(): [
  CodeFragment[],
  (...items: CodeFragment[]) => number,
  (...items: CodeFragment[]) => number,
] {
  const frag: CodeFragment[] = []
  const push = frag.push.bind(frag)
  const unshift = frag.unshift.bind(frag)
  return [frag, push, unshift]
}

export type CodeFragmentDelimiters = [
  left: CodeFragments,
  right: CodeFragments,
  delimiter: CodeFragments,
  placeholder?: CodeFragments,
]

export function genMulti(
  [left, right, seg, placeholder]: CodeFragmentDelimiters,
  ...frags: CodeFragments[]
): CodeFragment[] {
  if (placeholder) {
    while (frags.length > 0 && !frags.at(-1)) {
      frags.pop()
    }
    frags = frags.map((frag) => frag || placeholder)
  } else {
    frags = frags.filter(Boolean)
  }

  const frag: CodeFragment[] = []
  push(left)
  for (const [i, fn] of (
    frags as Array<Exclude<CodeFragments, FalsyValue>>
  ).entries()) {
    push(fn)
    if (i < frags.length - 1) push(seg)
  }
  push(right)
  return frag

  function push(fn: CodeFragments) {
    if (!isArray(fn)) fn = [fn]
    frag.push(...fn)
  }
}
export const DELIMITERS_ARRAY: CodeFragmentDelimiters = ['[', ']', ', ']
export const DELIMITERS_ARRAY_NEWLINE: CodeFragmentDelimiters = [
  ['[', INDENT_START, NEWLINE],
  [INDENT_END, NEWLINE, ']'],
  [',', NEWLINE],
]
export const DELIMITERS_OBJECT: CodeFragmentDelimiters = ['{ ', ' }', ', ']
export const DELIMITERS_OBJECT_NEWLINE: CodeFragmentDelimiters = [
  ['{', INDENT_START, NEWLINE],
  [INDENT_END, NEWLINE, '}'],
  [',', NEWLINE],
]

export function genCall(
  name: string | [name: string, placeholder?: CodeFragments],
  ...frags: CodeFragments[]
): CodeFragment[] {
  const hasPlaceholder = isArray(name)
  const fnName = hasPlaceholder ? name[0] : name
  const placeholder = hasPlaceholder ? name[1] : 'null'
  return [fnName, ...genMulti(['(', ')', ', ', placeholder], ...frags)]
}

export function codeFragmentToString(
  code: CodeFragment[],
  context: CodegenContext,
): [code: string, map: CodegenSourceMapGenerator | undefined] {
  const {
    options: { filename, sourceMap },
  } = context

  let map: CodegenSourceMapGenerator | undefined
  if (sourceMap) {
    // lazy require source-map implementation, only in non-browser builds
    map = new SourceMapGenerator() as unknown as CodegenSourceMapGenerator
    map.setSourceContent(filename, context.ir.source)
    map._sources.add(filename)
  }

  let codegen = ''
  const pos = { line: 1, column: 0, index: 0 }
  let indentLevel = 0

  for (let frag of code) {
    if (!frag) continue

    if (frag === NEWLINE) {
      frag = [`\n${`  `.repeat(indentLevel)}`, NewlineType.Start]
    } else if (frag === INDENT_START) {
      indentLevel++
      continue
    } else if (frag === INDENT_END) {
      indentLevel--
      continue
    }

    if (isString(frag)) frag = [frag]

    let [code, newlineIndex = NewlineType.None, loc, name] = frag
    codegen += code

    if (map) {
      // @ts-ignore TODO
      if (loc) addMapping(loc.start, name)
      if (newlineIndex === NewlineType.Unknown) {
        // multiple newlines, full iteration
        advancePositionWithMutation(pos, code)
      } else {
        // fast paths
        pos.index += code.length
        if (newlineIndex === NewlineType.None) {
          pos.column += code.length
        } else {
          // single newline at known index
          if (newlineIndex === NewlineType.End) {
            newlineIndex = code.length - 1
          }
          pos.line++
          pos.column = code.length - newlineIndex
        }
      }
      if (loc && loc !== locStub) {
        // @ts-ignore TODO
        addMapping(loc.end)
      }
    }
  }

  return [codegen, map]

  function addMapping(loc: Position, name: string | null = null) {
    // we use the private property to directly add the mapping
    // because the addMapping() implementation in source-map-js has a bunch of
    // unnecessary arg and validation checks that are pure overhead in our case.
    const { _names, _mappings } = map!
    if (name !== null && !_names.has(name)) _names.add(name)
    _mappings.add({
      originalLine: loc.line,
      originalColumn: loc.column,
      generatedLine: pos.line,
      generatedColumn: pos.column - 1,
      source: filename,
      name,
    })
  }
}

export function advancePositionWithMutation(
  pos: Position,
  source: string,
  numberOfCharacters: number = source.length,
): Position {
  let linesCount = 0
  let lastNewLinePos = -1
  for (let i = 0; i < numberOfCharacters; i++) {
    if (source.charCodeAt(i) === 10 /* newline char code */) {
      linesCount++
      lastNewLinePos = i
    }
  }

  pos.index += numberOfCharacters
  pos.line += linesCount
  pos.column =
    lastNewLinePos === -1
      ? pos.column + numberOfCharacters
      : numberOfCharacters - lastNewLinePos

  return pos
}

export function advancePositionWithClone(
  pos: Position,
  source: string,
  numberOfCharacters: number = source.length,
): Position {
  return advancePositionWithMutation(
    {
      index: pos.index,
      line: pos.line,
      column: pos.column,
    },
    source,
    numberOfCharacters,
  )
}
