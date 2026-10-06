# frozen_string_literal: true

module OpenhubBo
  module Core
    NATIVE = NativeBridge.new(Native)
    TOKEN = Op.new(NATIVE, "token")

    # Credentials, token cache and execution of operations. Share one session
    # between product clients (QrClient.new(session), PixClient.new(session)...)
    # so they reuse the OAuth token. Thread-safe.
    class Session
      attr_reader :environment

      def initialize(client_id, client_secret = nil, basic_token: nil, environment: Environment::SANDBOX,
                     base_url: nil, timeout: 30, transport: nil)
        @environment = environment.to_sym
        unless Environment::ALL.include?(@environment)
          raise ValidationError.new("must be one of #{Environment::ALL.join(', ')}", field: "environment")
        end

        @config = {
          "client_id" => client_id,
          "client_secret" => client_secret,
          "basic_token" => basic_token,
          "environment" => @environment.to_s,
          "base_url" => base_url
        }
        @transport = transport || NetHttpTransport.new(timeout: timeout)
        @token = nil
        @lock = Mutex.new
        # Fail fast on missing credentials instead of at the first request.
        TOKEN.build(@config, nil, token_input)
      end

      # Runs `op`: token, build, send, parse; refreshes a rejected token once.
      def execute(op, input)
        2.times do |attempt|
          token = ensure_token
          response = deliver(op, op.build(@config, token, input))
          begin
            return op.parse(input, response)
          rescue AuthenticationError
            raise if attempt == 1

            invalidate(token)
          end
        end
      end

      def close = @transport.close

      def inspect = "#<#{self.class.name} client_id=#{@config['client_id'].inspect} environment=#{@environment}>"
      alias_method :to_s, :inspect

      private

      def ensure_token
        @lock.synchronize do
          token = @token
          return token if token && Time.now.to_i < token["expires_at"]

          @token = TOKEN.parse(token_input, deliver(TOKEN, TOKEN.build(@config, nil, token_input)))
        end
      end

      def invalidate(token)
        @lock.synchronize { @token = nil if @token && @token["access_token"] == token["access_token"] }
      end

      def deliver(op, request)
        @transport.perform(request)
      rescue TransportError => e
        raise AmbiguousOutcomeError.new(op.name, e) if e.maybe_sent && !op.idempotent?

        raise
      end

      def token_input = { "now" => Time.now.to_i }
    end
  end
end
