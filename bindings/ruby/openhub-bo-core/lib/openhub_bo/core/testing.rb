# frozen_string_literal: true

require "securerandom"
require "openhub_bo/core"

module OpenhubBo
  module Core
    module Testing
      # In-memory fake of the OpenHub gateway (Sensedia): handles the OAuth token
      # and the access_token/client_id check like the real gateway; tests or
      # product gems register routes on top. Use it as a Session transport:
      #
      #   gateway = MockGateway.new
      #   gateway.route("GET", %r{/qr/simple/v2/verify/(\d+)\z}) { |req, m| Response.new(status: 200, body: "...") }
      #   session = gateway.session
      class MockGateway
        BASE_URL = "https://openhub.mock"
        TOKEN_PATH = "/oauth-client-credentials/access-token"

        attr_reader :client_id, :client_secret, :requests, :token_requests

        def initialize(client_id: "test-client", client_secret: "test-secret", token_ttl: 3600)
          @client_id = client_id
          @client_secret = client_secret
          @token_ttl = token_ttl
          @requests = []
          @token_requests = 0
          @tokens = []
          @routes = []
        end

        def route(method, pattern, &handler)
          @routes << [method.to_s.upcase, pattern, handler]
          self
        end

        # Answers matching requests with a fixed status and body.
        def stub(method, pattern, status:, body:)
          route(method, pattern) { Response.new(status: status, body: body) }
        end

        def session(client_secret: @client_secret, **kwargs)
          Session.new(@client_id, client_secret, base_url: BASE_URL, transport: self, **kwargs)
        end

        def revoke_tokens = @tokens.clear

        def perform(request)
          @requests << request
          path = URI(request["url"]).path
          return token_response(request) if path.end_with?(TOKEN_PATH)
          return Response.new(status: 401, body: "Access Token in the request, identified by HEADER access_token, is invalid.") unless authorized?(request)

          @routes.each do |method, pattern, handler|
            next unless request["method"] == method && (match = pattern.match(path))

            return handler.call(request, match)
          end
          Response.new(status: 404, body: JSON.generate({ "success" => false, "message" => "Not found", "errors" => [] }))
        end

        def close; end

        private

        def token_response(request)
          @token_requests += 1
          expected = "Basic #{["#{@client_id}:#{@client_secret}"].pack("m0")}"
          unless request["headers"]["Authorization"] == expected
            return Response.new(status: 401, body: JSON.generate({ "error" => "invalid_client", "error_description" => "Bad credentials" }))
          end

          token = SecureRandom.uuid
          @tokens << token
          # Sandbox answers 201 with token_type "access_token" and no scope.
          Response.new(status: 201, body: JSON.generate({ "access_token" => token, "token_type" => "access_token", "expires_in" => @token_ttl }))
        end

        def authorized?(request)
          @tokens.include?(request["headers"]["access_token"]) && request["headers"]["client_id"] == @client_id
        end
      end
    end
  end
end
