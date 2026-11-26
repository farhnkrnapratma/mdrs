import { load } from "./lazy.js";

await load();

export {
  FILE_EXTENSION,
  isMdrsPath,
  prepareMarkdownPage,
  prepareMarkdownPageWithData,
  renderMarkdown,
  renderMarkdownWithData,
  resolveDefines,
  resolveDefinesWithData,
} from "./lazy.js";
