# frozen_string_literal: true

# Merchant accounts at Red Enlace (ATC) OpenHub: lookup, creation, status,
# balances and movements for reconciliation.
require "openhub_bo/core"
begin
  # Precompiled gems ship one binary per Ruby minor version.
  require "openhub_bo/accounts/#{RUBY_VERSION[/\d+\.\d+/]}/openhub_bo_accounts"
rescue LoadError
  require "openhub_bo/accounts/openhub_bo_accounts"
end
require_relative "accounts/models"
require_relative "accounts/ops"
require_relative "accounts/client"
