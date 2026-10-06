# frozen_string_literal: true

# Payouts from your Red Enlace (ATC) OpenHub account: pay third-party
# interoperable QRs and send ACH transfer batches. Moves real money in production.
require "securerandom"
require "openhub_bo/core"
require "openhub_bo/payouts/native.so"
require_relative "payouts/models"
require_relative "payouts/ops"
require_relative "payouts/client"
