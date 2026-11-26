import init from "./wasm/mdrs.js";

export { FILE_EXTENSION, isMdrsPath } from "./path.js";
export {
  prepareMarkdownPage,
  prepareMarkdownPageWithData,
  renderMarkdown,
  renderMarkdownWithData,
  resolveDefines,
  resolveDefinesWithData,
} from "./wasm/mdrs.js";

let ready;

export function load(input) {
  ready ??= init(input === undefined ? undefined : { module_or_path: input }).then(
    () => undefined,
    (error) => {
      ready = undefined;
      throw error;
    },
  );
  return ready;
}
