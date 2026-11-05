export const FILE_EXTENSION = 'mdrs';

export function isMdrsPath(path) {
    const name = String(path).split(/[\\/]/).pop() ?? '';
    const dot = name.lastIndexOf('.');
    return dot > 0 && name.slice(dot + 1).toLowerCase() === FILE_EXTENSION;
}

export {
    prepareMarkdownPage,
    prepareMarkdownPageWithData,
    renderMarkdown,
    renderMarkdownWithData,
    resolveDefines,
    resolveDefinesWithData,
} from './index.native.js';
