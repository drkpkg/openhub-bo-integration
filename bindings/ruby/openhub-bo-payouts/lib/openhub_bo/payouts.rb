# frozen_string_literal: true

# Payouts from your Red Enlace (ATC) OpenHub account: pay third-party
# interoperable QRs and send ACH transfer batches. Moves real money in production.
require "securerandom"
require "openhub_bo/core"
begin
  # Precompiled gems ship one binary per Ruby minor version.
  require "openhub_bo/payouts/#{RUBY_VERSION[/\d+\.\d+/]}/openhub_bo_payouts"
rescue LoadError
  require "openhub_bo/payouts/openhub_bo_payouts"
end
require_relative "payouts/models"
require_relative "payouts/ops"
require_relative "payouts/client"
