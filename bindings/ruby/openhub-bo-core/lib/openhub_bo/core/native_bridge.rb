# frozen_string_literal: true

module OpenhubBo
  module Core
    # A native extension exposing `call(op, payload) -> String`. Checks the
    # protocol on load and caches the operation catalog from `describe`, so
    # declared operations are validated when the gem is required.
    class NativeBridge
      attr_reader :operations, :version

      def initialize(mod)
        protocol = mod.protocol_version
        unless protocol == SUPPORTED_PROTOCOL
          raise LoadError,
                "#{mod} speaks core protocol #{protocol}, this gem expects #{SUPPORTED_PROTOCOL}; " \
                "install matching openhub-bo versions"
        end
        @mod = mod
        @version = mod.version
        @operations = invoke("describe", {}).to_h { |entry| [entry["name"], entry] }.freeze
      end

      def invoke(op, payload)
        result = JSON.parse(@mod.call(op, JSON.generate(payload)))
        return result["value"] if result["ok"]

        raise Core.error_from_core(result["error"], op)
      end
    end
  end
end
