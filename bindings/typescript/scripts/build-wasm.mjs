// Builds native crates to wasm32 and generates Node bindings into
// packages/<pkg>/wasm with wasm-bindgen (target: nodejs, server-side only).
// With no arguments every package is built; with arguments only those packages
// (and core, which their tests import) are built.
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const allPackages = ["core", "qr", "fx", "accounts", "payouts"];

// `node scripts/build-wasm.mjs` builds every package. Extra args select a subset
// (`node scripts/build-wasm.mjs qr`), used by the release matrix. Dependents
// import @openhub-bo/core, which loads its wasm module, so core is always built.
const requested = process.argv.slice(2);
for (const pkg of requested) {
  if (!allPackages.includes(pkg)) {
    console.error(`unknown package: ${pkg} (expected ${allPackages.join(", ")})`);
    process.exit(1);
  }
}
const selected = requested.length === 0 ? allPackages : requested;
const wasmPackages = [
  ...new Set(selected.flatMap((pkg) => (pkg === "core" ? ["core"] : ["core", pkg]))),
];

function bin(name) {
  const local = join(homedir(), ".cargo", "bin", name);
  return existsSync(local) ? local : name;
}

execFileSync(
  bin("cargo"),
  [
    "build",
    "--release",
    "--target",
    "wasm32-unknown-unknown",
    ...wasmPackages.flatMap((pkg) => ["-p", `openhub-bo-${pkg}-wasm`]),
  ],
  { cwd: root, stdio: "inherit" },
);

for (const pkg of wasmPackages) {
  const wasm = join(root, "target/wasm32-unknown-unknown/release", `openhub_bo_${pkg}_wasm.wasm`);
  const out = join(root, "packages", pkg, "wasm");
  rmSync(out, { recursive: true, force: true });
  mkdirSync(out, { recursive: true });
  execFileSync(bin("wasm-bindgen"), [wasm, "--target", "nodejs", "--out-dir", out], { stdio: "inherit" });
  // wasm-bindgen's nodejs target is CommonJS; the packages themselves are ESM.
  writeFileSync(join(out, "package.json"), JSON.stringify({ type: "commonjs" }) + "\n");
  console.log(`built ${pkg} -> packages/${pkg}/wasm`);
}
