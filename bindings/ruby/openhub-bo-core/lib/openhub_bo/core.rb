# frozen_string_literal: true

# Shared foundation of the openhub-bo gems: session, transport, errors and
# common types for the Red Enlace (ATC) OpenHub APIs.
require "bigdecimal"
require "json"

require_relative "core/version"
require_relative "core/errors"
require_relative "core/models"
require_relative "core/native_bridge"
require_relative "core/op"
require_relative "core/transport"
begin
  # Precompiled gems ship one binary per Ruby minor version.
  require "openhub_bo/core/#{RUBY_VERSION[/\d+\.\d+/]}/openhub_bo_core"
rescue LoadError
  require "openhub_bo/core/openhub_bo_core"
end
require_relative "core/session"
