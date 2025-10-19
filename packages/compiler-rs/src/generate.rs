use napi_derive::napi;

#[napi(object)]
pub struct CodegenOptions {
  /**
   * Generate source map?
   * @default false
   */
  pub source_map: Option<bool>,
  /**
   * Filename for source map generation.
   * Also used for self-recursive reference in templates
   * @default 'index.jsx'
   */
  pub filename: Option<String>,
  pub templates: Option<Vec<String>>,
}
