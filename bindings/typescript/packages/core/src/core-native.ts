import { createRequire } from "node:module";

import { Native, type NativeModule } from "./native.js";

const require = createRequire(import.meta.url);

/** `@openhub-bo/core`'s own native module (OAuth token operation). */
export const CORE_NATIVE = new Native(
  require("../wasm/openhub_bo_core_wasm.js") as NativeModule,
  "@openhub-bo/core",
);
