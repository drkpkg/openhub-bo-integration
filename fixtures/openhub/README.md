# OpenHub fixtures

Shared by the Rust core tests and every language binding.

- `*_response.json` without a sandbox note: copied verbatim from the public docs
  (<https://openhub.redenlace.com.bo/es/products/apis-de-cobro-mediante-qr/cobro-por-qr-simple>),
  2026-10-06. Token value and QR image shortened.
- Captured from the **sandbox** on 2026-10-06 (real responses):
  `cancel_response.json` (HTTP 200), `cancel_invalid_state_response.json` (HTTP 409),
  `generate_validation_error_response.json` (HTTP 400).
