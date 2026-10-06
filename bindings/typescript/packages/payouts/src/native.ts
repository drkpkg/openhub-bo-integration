import { createRequire } from "node:module";

import { Native, type NativeModule } from "@openhub-bo/core";

const require = createRequire(import.meta.url);

export const NATIVE = new Native(
  require("../wasm/openhub_bo_payouts_wasm.js") as NativeModule,
  "@openhub-bo/payouts",
);
