import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const native = await import("../index.js");
const wasm = await import("../lazy.js");

await wasm.load(readFileSync(join(root, "wasm/mdrs_bg.wasm")));

const documents = [
  "",
  "# Title\n\nPlain **bold** and _em_ with `code` and a [link](https://outside.example/x).",
  ":::define\nproject: \"MDRS\"\ncount: 3\n:::\n\n## ${project}\n\nThere are ${count} items.",
  ":::note\nA note.\n:::\n\n:::warning\nCareful.\n:::\n\n:::details summary=\"More\"\nHidden.\n:::",
  ":::tabs\n::Rust\n```rust\nfn main() {}\n```\n::JavaScript\n```js\nconsole.log(1);\n```\n:::",
  "Inline $e^{i\\pi}+1=0$ and block:\n\n$$\n\\int_0^1 x\\,dx\n$$",
  "```mermaid\nflowchart LR\n  A[Start] --> B{Choice}\n  B -->|yes| C\n```\n\n```mermaid\nnot a diagram\n```",
  "```chart\ntype: bar\ndata: [1, 2, 3]\n```",
  "- [x] done\n- [ ] todo\n- [-] dropped\n\n| a | b |\n| - | - |\n| 1 | 2 |\n\nNote[^1].\n\n[^1]: Footnote.",
  "<script>alert(1)</script><a href=\"javascript:alert(1)\">x</a><b onclick=\"x\">y</b>",
  "## One\n\n### Two\n\n[jump](#two)\n\n## Café \u{1F600}",
];
const optionSets = [
  undefined,
  { internalOrigins: ["https://outside.example"] },
  { diagrams: "placeholder" },
  { diagrams: "pelers", internalOrigins: [] },
];
const data = JSON.stringify({ name: "Ada", items: [1, 2] });

let checks = 0;
const same = (call) => {
  let expected;
  let actual;
  try {
    expected = { value: call(native) };
  } catch (error) {
    expected = { error: String(error.message ?? error) };
  }
  try {
    actual = { value: call(wasm) };
  } catch (error) {
    actual = { error: String(error.message ?? error) };
  }
  assert.deepEqual(actual, expected);
  checks += 1;
};

for (const source of documents) {
  same((api) => api.resolveDefines(source));
  same((api) => api.resolveDefinesWithData(source, data));
  for (const options of optionSets) {
    same((api) => api.renderMarkdown(source, options));
    same((api) => api.renderMarkdownWithData(source, data, options));
    same((api) => structuredClone(api.prepareMarkdownPage(source, options)));
    same((api) => structuredClone(api.prepareMarkdownPageWithData(source, data, options)));
  }
}
same((api) => api.renderMarkdown("x", { diagrams: "svg" }));
same((api) => api.renderMarkdownWithData("x", "{not json"));
same((api) => api.renderMarkdown("Hello ${missing}"));
for (const path of ["a/b.mdrs", "C.MDRS", ".mdrs", "a.md"]) same((api) => api.isMdrsPath(path));
assert.equal(wasm.FILE_EXTENSION, native.FILE_EXTENSION);

assert.match(native.renderMarkdown(documents[6]), /<svg[^>]*class="pelers pelers-flowchart"/);

console.log(`wasm matches the native addon on ${checks} checks`);
