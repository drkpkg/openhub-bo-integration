// Builds every native crate to wasm32 and generates Node bindings into
// packages/<pkg>/wasm with wasm-bindgen (target: nodejs, server-side only).
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const packages = ["core", "qr", "fx", "accounts", "payouts"];

function bin(name) {
  const local = join(homedir(), ".cargo", "bin", name);
  return existsSync(local) ? local : name;
}

execFileSync(bin("cargo"), ["build", "--release", "--target", "wasm32-unknown-unknown"], {
  cwd: root,
  stdio: "inherit",
});

for (const pkg of packages) {
  const wasm = join(root, "target/wasm32-unknown-unknown/release", `openhub_bo_${pkg}_wasm.wasm`);
  const out = join(root, "packages", pkg, "wasm");
  rmSync(out, { recursive: true, force: true });
  mkdirSync(out, { recursive: true });
  execFileSync(bin("wasm-bindgen"), [wasm, "--target", "nodejs", "--out-dir", out], { stdio: "inherit" });
  // wasm-bindgen's nodejs target is CommonJS; the packages themselves are ESM.
  writeFileSync(join(out, "package.json"), JSON.stringify({ type: "commonjs" }) + "\n");
  console.log(`built ${pkg} -> packages/${pkg}/wasm`);
}
