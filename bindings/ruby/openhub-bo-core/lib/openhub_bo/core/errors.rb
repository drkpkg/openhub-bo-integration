# frozen_string_literal: true

module OpenhubBo
  module Core
    ApiFieldError = Data.define(:message, :field, :code)

    # Base class for every error raised by openhub-bo.
    class Error < StandardError
      # Whether repeating the same call later may succeed.
      def retryable? = false
    end

    # Input rejected locally, before calling OpenHub.
    class ValidationError < Error
      attr_reader :field

      def initialize(message, field:)
        super("invalid `#{field}`: #{message}")
        @field = field
      end
    end

    # Credentials or access token rejected by OpenHub.
    class AuthenticationError < Error
      attr_reader :status

      def initialize(message, status:)
        super("authentication failed (HTTP #{status}): #{message}")
        @status = status
      end
    end

    # OpenHub answered with an error.
    class ApiError < Error
      attr_reader :status, :errors

      def initialize(message, status:, errors: [], retryable: false)
        super("OpenHub error (HTTP #{status}): #{message}")
        @status = status
        @errors = errors
        @retryable = retryable
      end

      def retryable? = @retryable

      # Code of the first field error, e.g. "TRANSACCION_NO_ENCONTRADA".
      def code = errors.map(&:code).compact.first
    end

    # The response could not be understood.
    class DecodeError < Error; end

    # Incoming webhook did not carry the expected secret.
    class WebhookAuthError < Error; end

    # Network failure. `maybe_sent` is false when the request never left
    # (connection refused, connect timeout): retrying is then always safe.
    class TransportError < Error
      attr_reader :maybe_sent

      def initialize(message, maybe_sent:)
        super(message)
        @maybe_sent = maybe_sent
      end

      def retryable? = true
    end

    # A non-idempotent call (generating a QR, paying a payout...) may or may not
    # have been applied: the response was lost, or OpenHub reported the result as
    # unconfirmed (payout codes 94/96). Deliberately not a TransportError so
    # generic retry loops do not repeat it. Query the status before retrying.
    class AmbiguousOutcomeError < Error
      attr_reader :operation, :errors

      def initialize(operation, cause, errors: [])
        super("outcome of `#{operation}` is unknown: #{cause}")
        @operation = operation
        @errors = errors
      end

      def code = errors.map(&:code).compact.first
    end

    # Contract violation between a gem and its native extension (a bug).
    class CoreError < Error; end

    module_function

    # Maps an error from the native core; `op` is the native op name
    # (e.g. "payouts.pay.parse").
    def error_from_core(data, op = "")
      kind = data["kind"]
      message = data.fetch("message", "unknown error")
      errors = Array(data["errors"]).map do |e|
        ApiFieldError.new(message: e["message"], field: e["field"], code: e["code"])
      end
      case kind
      when "validation" then ValidationError.new(message, field: data["field"])
      when "authentication" then AuthenticationError.new(message, status: data["status"])
      when "api"
        ApiError.new(message, status: data["status"], errors: errors, retryable: data["retryable"] == true)
      when "ambiguous"
        AmbiguousOutcomeError.new(op.sub(/\.(build|parse)\z/, ""), message, errors: errors)
      when "decode" then DecodeError.new(message)
      when "webhook_auth" then WebhookAuthError.new(message)
      else CoreError.new(message)
      end
    end
  end
end
