# frozen_string_literal: true

# Cross-currency QR collections for the Red Enlace (ATC) OpenHub APIs: PIX
# (payers in Brazil), virtual assets via Koibanx (USDT/USDC) and Binance Pay.
require "openhub_bo/core"
begin
  # Precompiled gems ship one binary per Ruby minor version.
  require "openhub_bo/fx/#{RUBY_VERSION[/\d+\.\d+/]}/openhub_bo_fx"
rescue LoadError
  require "openhub_bo/fx/openhub_bo_fx"
end
require_relative "fx/models"
require_relative "fx/ops"
require_relative "fx/client"
