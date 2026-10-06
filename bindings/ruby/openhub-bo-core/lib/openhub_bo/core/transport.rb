# frozen_string_literal: true

require "net/http"
require "openssl"
require "uri"

module OpenhubBo
  module Core
    Response = Data.define(:status, :body) do
      def to_wire = { "status" => status, "body" => body }
    end

    # Sends requests built by the native core. Any object responding to
    # `perform(request) -> Response` works: inject your own for proxies,
    # logging or fakes. `request` is {"method", "url", "headers", "body",
    # "timeout"?}; raise TransportError on network failures.
    class NetHttpTransport
      # Failures where the request provably never reached the server.
      NOT_SENT = [
        Errno::ECONNREFUSED, Errno::EHOSTUNREACH, Errno::ENETUNREACH, Net::OpenTimeout, SocketError
      ].freeze
      MAYBE_SENT = [
        Net::ReadTimeout, Net::WriteTimeout, EOFError, IOError, Errno::ECONNRESET, Errno::EPIPE,
        OpenSSL::SSL::SSLError
      ].freeze

      def initialize(timeout: 30, open_timeout: 10)
        @timeout = timeout
        @open_timeout = open_timeout
      end

      def perform(request)
        uri = URI(request["url"])
        http = Net::HTTP.new(uri.host, uri.port)
        http.use_ssl = uri.scheme == "https"
        http.open_timeout = @open_timeout
        http.read_timeout = request["timeout"] || @timeout
        http.write_timeout = request["timeout"] || @timeout
        body = request["body"]
        message = Net::HTTPGenericRequest.new(
          request["method"], !body.nil?, true, uri.request_uri, request["headers"]
        )
        message.body = body if body
        response = http.start { |conn| conn.request(message) }
        Response.new(status: response.code.to_i, body: response.body.to_s.dup.force_encoding(Encoding::UTF_8))
      rescue *NOT_SENT => e
        raise TransportError.new("#{e.class}: #{e.message}", maybe_sent: false)
      rescue *MAYBE_SENT => e
        raise TransportError.new("#{e.class}: #{e.message}", maybe_sent: true)
      end

      def close; end
    end
  end
end
