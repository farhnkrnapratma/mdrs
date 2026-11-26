import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const target = "target/wasm";
const out = join(root, "wasm");

const run = (command, args) => execFileSync(command, args, { cwd: root, stdio: "inherit" });

function wasmOpt() {
  const cache = join(homedir(), ".cache/.wasm-pack");
  if (existsSync(cache))
    for (const entry of readdirSync(cache).filter((name) => name.startsWith("wasm-opt-"))) {
      const binary = join(cache, entry, "bin/wasm-opt");
      if (existsSync(binary)) return binary;
    }
  return "wasm-opt";
}

rmSync(out, { recursive: true, force: true });
run("cargo", [
  "build",
  "--lib",
  "--locked",
  "--release",
  "--target",
  "wasm32-unknown-unknown",
  "--features",
  "wasm",
  "--config",
  'profile.release.panic="abort"',
  "--config",
  'profile.release.opt-level="z"',
  "--config",
  'profile.release.lto="fat"',
  "--config",
  "profile.release.codegen-units=1",
  "--target-dir",
  target,
]);
run("wasm-bindgen", [
  "--target",
  "web",
  "--out-dir",
  out,
  "--out-name",
  "mdrs",
  `${target}/wasm32-unknown-unknown/release/mdrs.wasm`,
]);
const module = join(out, "mdrs_bg.wasm");
try {
  run(wasmOpt(), ["-Oz", "--all-features", "--strip-debug", module, "-o", module]);
} catch (error) {
  if (process.env.CI) throw error;
  console.warn("wasm-opt not found; the module is left unoptimized");
}
writeFileSync(join(out, "package.json"), `${JSON.stringify({ type: "module" })}\n`);
console.log(`wasm: ${readFileSync(module).length} bytes`);
