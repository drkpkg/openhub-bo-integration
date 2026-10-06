# frozen_string_literal: true

# Cross-currency QR collections for the Red Enlace (ATC) OpenHub APIs: PIX
# (payers in Brazil), virtual assets via Koibanx (USDT/USDC) and Binance Pay.
require "openhub_bo/core"
require "openhub_bo/fx/native.so"
require_relative "fx/models"
require_relative "fx/ops"
require_relative "fx/client"
