# frozen_string_literal: true

module OpenhubBo
  module Core
    # An HTTP endpoint implemented by a native extension. Declaring one with a
    # name the extension does not know raises LoadError at require time.
    class Op
      attr_reader :native, :name

      def initialize(native, name, &decode)
        info = native.operations[name]
        raise LoadError, "native extension has no operation `#{name}`" unless info && info["kind"] == "operation"

        @native = native
        @name = name
        @decode = decode || ->(value) { value }
        @info = info
      end

      def idempotent? = @info["idempotent"] == true

      # Client timeout (seconds) recommended by ATC for this endpoint, if any.
      def timeout = @info["timeout_secs"]

      def build(config, token, input)
        request = native.invoke("#{name}.build", { "config" => config, "token" => token, "input" => input })
        request["timeout"] = timeout if timeout
        request
      end

      def parse(input, response)
        @decode.call(native.invoke("#{name}.parse", { "input" => input, "response" => response.to_wire }))
      end
    end

    # A pure native function (no HTTP), e.g. webhook parsing.
    class Handler
      def initialize(native, name, &decode)
        info = native.operations[name]
        raise LoadError, "native extension has no handler `#{name}`" unless info && info["kind"] == "handler"

        @native = native
        @name = name
        @decode = decode || ->(value) { value }
      end

      def call(input) = @decode.call(@native.invoke(@name, input))
    end
  end
end
