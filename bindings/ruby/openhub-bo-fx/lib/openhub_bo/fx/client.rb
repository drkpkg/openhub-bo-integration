# frozen_string_literal: true

module OpenhubBo
  module Fx
    # Charge Brazilian payers via PIX: they pay BRL, you receive BOB/USD.
    # Requires PIX to be enabled for your merchant (otherwise ApiError GQ-00005).
    class PixClient
      attr_reader :session

      def initialize(session)
        @session = session
      end

      # `payer_phone` is "+" and 13 digits; `expires_in` defaults to 100 s on
      # OpenHub's side.
      def generate_qr(amount:, glosa:, reference:, payer_cpf:, payer_phone:, webhook:, currency: "BOB",
                      channel: "WEB", expires_in: nil, payer_email: nil, extra: nil)
        input = Ops.common(amount:, glosa:, reference:, webhook:, currency:, channel:).merge(
          "payer_cpf" => payer_cpf, "payer_phone" => payer_phone, "payer_email" => payer_email,
          "expires_in" => Ops.optional_seconds(expires_in), "extra" => extra
        )
        session.execute(Ops::PIX_GENERATE, input)
      end

      def get_status(reference) = session.execute(Ops::PIX_VERIFY, Ops.ref(reference))

      # OpenHub's "cancela". The sandbox only allows it on paid ("aprobado") QRs,
      # i.e. it acts as a refund request; unpaid ones answer EG-00001.
      def cancel(reference) = session.execute(Ops::PIX_CANCEL, Ops.ref(reference))
    end

    # Charge in BOB/USD, settle in USDT/USDC through Koibanx. Minimum Bs 50 in
    # BOB; `expires_in` 180..600 s (sandbox rules).
    class VirtualAssetsClient
      attr_reader :session

      def initialize(session)
        @session = session
      end

      def generate_qr(amount:, glosa:, reference:, asset:, webhook:, currency: "BOB", channel: "WEB",
                      expires_in: 180, extra: nil)
        unless VIRTUAL_ASSETS.include?(asset.to_sym)
          raise Core::ValidationError.new("must be one of #{VIRTUAL_ASSETS.join(', ')}", field: "asset")
        end

        input = Ops.common(amount:, glosa:, reference:, webhook:, currency:, channel:).merge(
          "asset" => asset.to_s, "expires_in" => Core.seconds(expires_in), "extra" => extra
        )
        session.execute(Ops::CRYPTO_GENERATE, input)
      end

      def get_status(reference) = session.execute(Ops::CRYPTO_VERIFY, Ops.ref(reference))
    end

    # Charge through Binance Pay QR. `reference` at most 10 digits;
    # `expires_in` at most 300 s.
    class BinanceClient
      attr_reader :session

      def initialize(session)
        @session = session
      end

      def generate_qr(amount:, glosa:, reference:, webhook:, currency: "BOB", channel: "WEB", expires_in: nil,
                      extra: nil)
        input = Ops.common(amount:, glosa:, reference:, webhook:, currency:, channel:).merge(
          "expires_in" => Ops.optional_seconds(expires_in), "extra" => extra
        )
        session.execute(Ops::BINANCE_GENERATE, input)
      end

      def get_status(reference) = session.execute(Ops::BINANCE_VERIFY, Ops.ref(reference))
    end

    module_function

    # Authenticates and parses a Binance Pay notification. Answer with HTTP 200
    # and `notification.ack` as JSON; confirm with BinanceClient#get_status.
    def parse_binance_webhook(headers, body, webhook:)
      Ops::PARSE_BINANCE_WEBHOOK.call(Ops.webhook_call(headers, body, webhook))
    end

    # Authenticates and reads a PIX / Koibanx notification (undocumented payload:
    # best-effort fields, full body in `payload`). Always confirm with get_status.
    def parse_fx_webhook(headers, body, webhook:)
      Ops::PARSE_FX_WEBHOOK.call(Ops.webhook_call(headers, body, webhook))
    end
  end
end
