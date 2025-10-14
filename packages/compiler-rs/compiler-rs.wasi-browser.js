import {
  createOnMessage as __wasmCreateOnMessageForFsProxy,
  getDefaultContext as __emnapiGetDefaultContext,
  instantiateNapiModuleSync as __emnapiInstantiateNapiModuleSync,
  WASI as __WASI,
} from '@napi-rs/wasm-runtime'



const __wasi = new __WASI({
  version: 'preview1',
})

const __wasmUrl = new URL('./compiler-rs.wasm32-wasi.wasm', import.meta.url).href
const __emnapiContext = __emnapiGetDefaultContext()


const __sharedMemory = new WebAssembly.Memory({
  initial: 4000,
  maximum: 65536,
  shared: true,
})

const __wasmFile = await fetch(__wasmUrl).then((res) => res.arrayBuffer())

const {
  instance: __napiInstance,
  module: __wasiModule,
  napiModule: __napiModule,
} = __emnapiInstantiateNapiModuleSync(__wasmFile, {
  context: __emnapiContext,
  asyncWorkPoolSize: 4,
  wasi: __wasi,
  onCreateWorker() {
    const worker = new Worker(new URL('./wasi-worker-browser.mjs', import.meta.url), {
      type: 'module',
    })

    return worker
  },
  overwriteImports(importObject) {
    importObject.env = {
      ...importObject.env,
      ...importObject.napi,
      ...importObject.emnapi,
      memory: __sharedMemory,
    }
    return importObject
  },
  beforeInit({ instance }) {
    for (const name of Object.keys(instance.exports)) {
      if (name.startsWith('__napi_register__')) {
        instance.exports[name]()
      }
    }
  },
})
export default __napiModule.exports
export const camelize = __napiModule.exports.camelize
export const createBranch = __napiModule.exports.createBranch
export const createCompilerError = __napiModule.exports.createCompilerError
export const createSimpleExpression = __napiModule.exports.createSimpleExpression
export const DynamicFlag = __napiModule.exports.DynamicFlag
export const EMPTY_EXPRESSION = __napiModule.exports.EMPTY_EXPRESSION
export const ErrorCodes = __napiModule.exports.ErrorCodes
export const findProp = __napiModule.exports.findProp
export const getExpression = __napiModule.exports.getExpression
export const getLiteralExpressionValue = __napiModule.exports.getLiteralExpressionValue
export const getText = __napiModule.exports.getText
export const getTextLikeValue = __napiModule.exports.getTextLikeValue
export const IRDynamicPropsKind = __napiModule.exports.IRDynamicPropsKind
export const IRNodeTypes = __napiModule.exports.IRNodeTypes
export const IRSlotType = __napiModule.exports.IRSlotType
export const isBigIntLiteral = __napiModule.exports.isBigIntLiteral
export const isBlockOperation = __napiModule.exports.isBlockOperation
export const isConstantExpression = __napiModule.exports.isConstantExpression
export const isConstantNode = __napiModule.exports.isConstantNode
export const isEmptyText = __napiModule.exports.isEmptyText
export const isFragmentNode = __napiModule.exports.isFragmentNode
export const isJSXComponent = __napiModule.exports.isJSXComponent
export const isNumericLiteral = __napiModule.exports.isNumericLiteral
export const isStringLiteral = __napiModule.exports.isStringLiteral
export const isTemplate = __napiModule.exports.isTemplate
export const locStub = __napiModule.exports.locStub
export const LOC_STUB = __napiModule.exports.LOC_STUB
export const newBlock = __napiModule.exports.newBlock
export const newDynamic = __napiModule.exports.newDynamic
export const processConditionalExpression = __napiModule.exports.processConditionalExpression
export const processLogicalExpression = __napiModule.exports.processLogicalExpression
export const resolveDirective = __napiModule.exports.resolveDirective
export const resolveExpression = __napiModule.exports.resolveExpression
export const resolveJSXText = __napiModule.exports.resolveJSXText
export const transformNode = __napiModule.exports.transformNode
export const transformTemplateRef = __napiModule.exports.transformTemplateRef
export const transformVBind = __napiModule.exports.transformVBind
export const transformVFor = __napiModule.exports.transformVFor
export const transformVHtml = __napiModule.exports.transformVHtml
export const transformVIf = __napiModule.exports.transformVIf
export const transformVOnce = __napiModule.exports.transformVOnce
export const transformVShow = __napiModule.exports.transformVShow
export const transformVSlot = __napiModule.exports.transformVSlot
export const transformVSlots = __napiModule.exports.transformVSlots
export const transformVText = __napiModule.exports.transformVText
export const TS_NODE_TYPES = __napiModule.exports.TS_NODE_TYPES
export const unwrapTSNode = __napiModule.exports.unwrapTSNode
export const wrapFragment = __napiModule.exports.wrapFragment
