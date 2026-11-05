import { createHash } from 'node:crypto';
import { $ } from 'bun';
import { renderMarkdown } from '../../idstead/packages/mdrs/core/src/markdown';
import { jsToDefineValue, walkPath, type ApiRegistry } from '../../idstead/packages/mdrs/core/src/define';

const fixture = process.argv[2] ?? '../idstead/docs/008-mdrs/SHOWCASE.md';
const source = await Bun.file(fixture).text();
const author = jsToDefineValue({
    profile: { name: 'Ann Example', siteUrl: 'https://example.com' }
});
const blog = jsToDefineValue([
    { title: 'First post', slug: 'first' },
    { title: 'Second post', slug: 'second' },
    { title: 'Third post', slug: 'third' }
]);
const resolvers: ApiRegistry = {
    author: async path => walkPath(author, path),
    blog: async path => walkPath(blog, path)
};

const mirror = await renderMarkdown(source, resolvers, {
    internalOrigins: ['https://my-site.example']
});
const rust = await $`cargo run --quiet --locked --offline --example showcase_render -- ${fixture}`.text();
const mirrorHash = createHash('sha256').update(mirror).digest('hex');
const rustHash = createHash('sha256').update(rust).digest('hex');
const normalizedMirror = normalizeHtml(mirror);
const normalizedRust = normalizeHtml(rust);
const normalizedMirrorHash = createHash('sha256').update(normalizedMirror).digest('hex');
const normalizedRustHash = createHash('sha256').update(normalizedRust).digest('hex');

console.log(JSON.stringify({
    fixture,
    sourceBytes: Buffer.byteLength(source),
    mirrorBytes: Buffer.byteLength(mirror),
    rustBytes: Buffer.byteLength(rust),
    mirrorSha256: mirrorHash,
    rustSha256: rustHash,
    exactMatch: mirror === rust,
    normalizedMirrorSha256: normalizedMirrorHash,
    normalizedRustSha256: normalizedRustHash,
    normalizedMatch: normalizedMirror === normalizedRust,
    markers: {
        mirror: markerCounts(mirror),
        rust: markerCounts(rust)
    },
    firstDifference: firstDifference(mirror, rust),
    normalizedFirstDifference: firstDifference(normalizedMirror, normalizedRust)
}, null, 2));

function markerCounts(html: string) {
    return Object.fromEntries(
        ['mdrs-math', 'mdrs-mermaid', 'mdrs-chart', 'mdrs-embed', 'mdrs-references', 'mdrs-task']
            .map(marker => [marker, html.split(marker).length - 1])
    );
}

function firstDifference(left: string, right: string) {
    const length = Math.min(left.length, right.length);
    for (let index = 0; index < length; index++) {
        if (left[index] !== right[index]) {
            return {
                offset: index,
                mirror: left.slice(Math.max(0, index - 80), index + 80),
                rust: right.slice(Math.max(0, index - 80), index + 80)
            };
        }
    }
    return left.length === right.length ? null : {
        offset: length,
        mirror: left.slice(length, length + 160),
        rust: right.slice(length, length + 160)
    };
}

function normalizeHtml(html: string) {
    const tokens = html.match(/<!--[\s\S]*?-->|<![^>]*>|<\/?[A-Za-z][^>]*>|[^<]+/g) ?? [];
    return tokens.map(token => {
        if (token.startsWith('<!--')) return token;
        if (token.startsWith('<!')) return token.replace(/\s+/g, ' ').trim();
        if (!token.startsWith('<')) return decodeEntities(token).replace(/\s+/g, ' ');

        if (/^<\//.test(token)) return `</${token.slice(2, -1).trim().toLowerCase()}>`;

        const selfClosing = /\/\s*>$/.test(token);
        const match = token.match(/^<\s*([A-Za-z][\w:-]*)([\s\S]*?)\s*\/?>$/);
        if (!match) return token.replace(/\s+/g, ' ').trim();

        const [, name, rawAttributes] = match;
        const attributes = [...rawAttributes.matchAll(
            /([^\s=/>]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?/g
        )].map(([, key, doubleQuoted, singleQuoted, unquoted]) => [
            key.toLowerCase(),
            decodeEntities(doubleQuoted ?? singleQuoted ?? unquoted ?? '')
        ] as const).sort(([left], [right]) => left.localeCompare(right));
        const serializedAttributes = attributes
            .map(([key, value]) => value === '' ? key : `${key}="${escapeAttribute(value)}"`)
            .join(' ');
        const opening = `<${name.toLowerCase()}${serializedAttributes ? ` ${serializedAttributes}` : ''}>`;
        return selfClosing ? `${opening}</${name.toLowerCase()}>` : opening;
    }).join('').replace(/>\s+</g, '><').trim();
}

function decodeEntities(value: string) {
    return value
        .replace(/&amp;/g, '&')
        .replace(/&lt;/g, '<')
        .replace(/&gt;/g, '>')
        .replace(/&quot;/g, '"')
        .replace(/&#39;|&apos;/g, "'");
}

function escapeAttribute(value: string) {
    return value
        .replace(/&/g, '&amp;')
        .replace(/"/g, '&quot;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;');
}
