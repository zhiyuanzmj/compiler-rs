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
export const _DynamicFlag = __napiModule.exports._DynamicFlag
export const camelize = __napiModule.exports.camelize
export const createSimpleExpression = __napiModule.exports.createSimpleExpression
export const EMPTY_EXPRESSION = __napiModule.exports.EMPTY_EXPRESSION
export const ErrorCodes = __napiModule.exports.ErrorCodes
export const extractIdentifiers = __napiModule.exports.extractIdentifiers
export const FragmentSymbol = __napiModule.exports.FragmentSymbol
export const genBlock = __napiModule.exports.genBlock
export const genBuiltinDirective = __napiModule.exports.genBuiltinDirective
export const genCall = __napiModule.exports.genCall
export const genCreateNodes = __napiModule.exports.genCreateNodes
export const genDeclareOldRef = __napiModule.exports.genDeclareOldRef
export const genDirectiveModifiers = __napiModule.exports.genDirectiveModifiers
export const genDirectivesForElement = __napiModule.exports.genDirectivesForElement
export const genDynamicProps = __napiModule.exports.genDynamicProps
export const genEffect = __napiModule.exports.genEffect
export const genEventHandler = __napiModule.exports.genEventHandler
export const genExpression = __napiModule.exports.genExpression
export const genGetTextChild = __napiModule.exports.genGetTextChild
export const genIf = __napiModule.exports.genIf
export const genInsertionState = __napiModule.exports.genInsertionState
export const genInsertNode = __napiModule.exports.genInsertNode
export const genModelHandler = __napiModule.exports.genModelHandler
export const genMulti = __napiModule.exports.genMulti
export const genOperation = __napiModule.exports.genOperation
export const genOperations = __napiModule.exports.genOperations
export const genOperationWithInsertionState = __napiModule.exports.genOperationWithInsertionState
export const genPropKey = __napiModule.exports.genPropKey
export const genPropValue = __napiModule.exports.genPropValue
export const genSelf = __napiModule.exports.genSelf
export const genSetDynamicEvents = __napiModule.exports.genSetDynamicEvents
export const genSetEvent = __napiModule.exports.genSetEvent
export const genSetHtml = __napiModule.exports.genSetHtml
export const genSetNodes = __napiModule.exports.genSetNodes
export const genSetProp = __napiModule.exports.genSetProp
export const genSetTemplateRef = __napiModule.exports.genSetTemplateRef
export const genSetText = __napiModule.exports.genSetText
export const genTemplates = __napiModule.exports.genTemplates
export const genVModel = __napiModule.exports.genVModel
export const genVShow = __napiModule.exports.genVShow
export const getDelimitersArray = __napiModule.exports.getDelimitersArray
export const getDelimitersArrayNewline = __napiModule.exports.getDelimitersArrayNewline
export const getDelimitersObject = __napiModule.exports.getDelimitersObject
export const getDelimitersObjectNewline = __napiModule.exports.getDelimitersObjectNewline
export const getExpression = __napiModule.exports.getExpression
export const getLiteralExpressionValue = __napiModule.exports.getLiteralExpressionValue
export const getTextLikeValue = __napiModule.exports.getTextLikeValue
export const IRNodeTypes = __napiModule.exports.IRNodeTypes
export const IRSlotType = __napiModule.exports.IRSlotType
export const isBigIntLiteral = __napiModule.exports.isBigIntLiteral
export const isBlockOperation = __napiModule.exports.isBlockOperation
export const isConstantExpression = __napiModule.exports.isConstantExpression
export const isConstantNode = __napiModule.exports.isConstantNode
export const isEmptyText = __napiModule.exports.isEmptyText
export const isFnExpression = __napiModule.exports.isFnExpression
export const isForStatement = __napiModule.exports.isForStatement
export const isFunctionType = __napiModule.exports.isFunctionType
export const isIdentifier = __napiModule.exports.isIdentifier
export const isInDestructureAssignment = __napiModule.exports.isInDestructureAssignment
export const isJSXComponent = __napiModule.exports.isJSXComponent
export const isMemberExpression = __napiModule.exports.isMemberExpression
export const isNumericLiteral = __napiModule.exports.isNumericLiteral
export const isReferenced = __napiModule.exports.isReferenced
export const isReferencedIdentifier = __napiModule.exports.isReferencedIdentifier
export const isSimpleIdentifier = __napiModule.exports.isSimpleIdentifier
export const isStaticProperty = __napiModule.exports.isStaticProperty
export const isStringLiteral = __napiModule.exports.isStringLiteral
export const locStub = __napiModule.exports.locStub
export const LOC_STUB = __napiModule.exports.LOC_STUB
export const NewlineType = __napiModule.exports.NewlineType
export const resolveJSXText = __napiModule.exports.resolveJSXText
export const toValidAssetId = __napiModule.exports.toValidAssetId
export const transform = __napiModule.exports.transform
export const TS_NODE_TYPES = __napiModule.exports.TS_NODE_TYPES
export const unwrapTSNode = __napiModule.exports.unwrapTSNode
export const walk = __napiModule.exports.walk
export const walkIdentifiers = __napiModule.exports.walkIdentifiers
