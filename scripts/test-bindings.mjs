import { readFileSync } from 'node:fs';
import { existsSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');

function read(relativePath) {
    return readFileSync(join(root, relativePath), 'utf8');
}

function check(condition, message) {
    if (!condition) {
        throw new Error(message);
    }
}

const packageJson = JSON.parse(read('package.json'));
const declarations = read('index.d.ts');
const nodeBinding = read('src/bindings/node.rs');
const wasmBinding = read('src/bindings/wasm.rs');

check(packageJson.exports?.['.']?.types === './index.d.ts', 'package exports must point to index.d.ts');
check(packageJson.files?.includes('index.d.ts'), 'package files must include index.d.ts');

for (const name of [
    'isMdrsPath',
    'renderMarkdown',
    'renderMarkdownWithData',
    'prepareMarkdownPage',
    'prepareMarkdownPageWithData',
    'resolveDefines',
    'resolveDefinesWithData'
]) {
    check(declarations.includes(`function ${name}(`), `missing declaration for ${name}`);
    check(nodeBinding.includes(`js_name = "${name}"`), `missing Node export for ${name}`);
    check(wasmBinding.includes(`js_name = ${name}`), `missing WASM export for ${name}`);
}

for (const feature of ['napi', 'wasm']) {
    const result = spawnSync(
        'cargo',
        ['check', '--locked', '--offline', '--features', feature],
        { cwd: root, encoding: 'utf8', stdio: 'inherit' }
    );
    if (result.error) {
        throw result.error;
    }
    check(result.status === 0, `offline cargo check failed for ${feature}`);
}

const wasmTarget = spawnSync(
    'cargo',
    ['check', '--locked', '--offline', '--target', 'wasm32-unknown-unknown', '--features', 'wasm'],
    { cwd: root, encoding: 'utf8', stdio: 'inherit' }
);
check(wasmTarget.status === 0, 'offline WASM target check failed');

const nodeRuntime = spawnSync(
    'cargo',
    ['test', '--locked', '--offline', '--features', 'napi', '--lib', 'bindings::node::tests'],
    { cwd: root, encoding: 'utf8', stdio: 'inherit' }
);
check(nodeRuntime.status === 0, 'Node binding API smoke test failed');

if (existsSync(join(root, 'index.native.js'))) {
    const packageEntry = await import(join(root, 'index.js'));
    check(typeof packageEntry.renderMarkdown === 'function', 'package must export native renderMarkdown');
    check(packageEntry.renderMarkdown('# Native') === '<h1>Native</h1>\n', 'native package render failed');
    check(packageEntry.isMdrsPath('docs/page.mdrs'), 'package utility export failed');
}

for (const tool of ['napi', 'wasm-pack']) {
    const probe = spawnSync(tool, ['--version'], { cwd: root, encoding: 'utf8' });
    console.log(probe.status === 0
        ? `${tool}: available`
        : `${tool}: unavailable; artifact packaging not attempted`);
}

console.log('Binding declarations, Node API smoke test, and offline feature builds validated.');
