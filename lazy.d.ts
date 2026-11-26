export * from "./index.js";

export type WasmSource =
  | WebAssembly.Module
  | BufferSource
  | Response
  | Promise<Response>
  | URL
  | string;

export declare function load(input?: WasmSource): Promise<void>;
