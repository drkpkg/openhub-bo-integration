# frozen_string_literal: true

# QR Simple / QR MLD-BCB collections for the Red Enlace (ATC) OpenHub APIs.
require "openhub_bo/core"
begin
  # Precompiled gems ship one binary per Ruby minor version.
  require "openhub_bo/qr/#{RUBY_VERSION[/\d+\.\d+/]}/openhub_bo_qr"
rescue LoadError
  require "openhub_bo/qr/openhub_bo_qr"
end
require_relative "qr/models"
require_relative "qr/ops"
require_relative "qr/client"
