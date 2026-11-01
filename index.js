// SPDX-FileCopyrightText: 2026 Farhan Kurnia Pratama
//
// SPDX-License-Identifier: MIT OR Apache-2.0

export const FILE_EXTENSION = 'mdrs';

export function isMdrsPath(path) {
    const name = String(path).split(/[\\/]/).pop() ?? '';
    const dot = name.lastIndexOf('.');
    return dot > 0 && name.slice(dot + 1).toLowerCase() === FILE_EXTENSION;
}
