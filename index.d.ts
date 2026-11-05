/** The file extension used by MDRS documents, without the leading dot. */
export declare const FILE_EXTENSION: 'mdrs';

/** Returns `true` when `path` ends in the `.mdrs` extension (case-insensitive). */
export declare function isMdrsPath(path: string): boolean;

export interface RenderOptions {
    internalOrigins?: string[];
}

export interface HeadingItem {
    id: string;
    text: string;
    level: number;
}

export interface PreparedMarkdownPage {
    html: string;
    headings: HeadingItem[];
}

export declare function renderMarkdown(
    markdown: string,
    options?: RenderOptions
): string;

export declare function renderMarkdownWithData(
    markdown: string,
    dataJson: string,
    options?: RenderOptions
): string;

export declare function prepareMarkdownPage(
    markdown: string,
    options?: RenderOptions
): PreparedMarkdownPage;

export declare function prepareMarkdownPageWithData(
    markdown: string,
    dataJson: string,
    options?: RenderOptions
): PreparedMarkdownPage;

export declare function resolveDefines(markdown: string): string;

export declare function resolveDefinesWithData(markdown: string, dataJson: string): string;
