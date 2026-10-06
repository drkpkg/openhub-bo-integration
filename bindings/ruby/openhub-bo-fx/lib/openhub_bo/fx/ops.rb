# frozen_string_literal: true

module OpenhubBo
  module Fx
    # Operations declared against the native extension (checked at require time).
    module Ops
      NATIVE = Core::NativeBridge.new(Fx::Native)
      PIX_GENERATE = Core::Op.new(NATIVE, "pix.generate") { FxQr.from_h(_1) }
      PIX_VERIFY = Core::Op.new(NATIVE, "pix.verify") { FxStatusInfo.from_h(_1) }
      PIX_CANCEL = Core::Op.new(NATIVE, "pix.cancel") { FxStatusInfo.from_h(_1) }
      CRYPTO_GENERATE = Core::Op.new(NATIVE, "crypto.generate") { FxQr.from_h(_1) }
      CRYPTO_VERIFY = Core::Op.new(NATIVE, "crypto.verify") { FxStatusInfo.from_h(_1) }
      BINANCE_GENERATE = Core::Op.new(NATIVE, "binance.generate") { FxQr.from_h(_1) }
      BINANCE_VERIFY = Core::Op.new(NATIVE, "binance.verify") { FxStatusInfo.from_h(_1) }
      PARSE_BINANCE_WEBHOOK = Core::Handler.new(NATIVE, "binance.webhook.parse") { BinanceNotification.from_h(_1) }
      PARSE_FX_WEBHOOK = Core::Handler.new(NATIVE, "fx.webhook.parse") { FxNotification.from_h(_1) }

      module_function

      def common(amount:, glosa:, reference:, webhook:, currency:, channel:)
        {
          "reference" => reference.to_s,
          "glosa" => glosa.respond_to?(:to_wire) ? glosa.to_wire : glosa.to_s,
          "amount" => Core.amount(amount),
          "currency" => currency(currency),
          "channel" => channel.to_s,
          "webhook" => webhook.to_wire
        }
      end

      def currency(value)
        code = value.to_s.upcase
        raise Core::ValidationError.new("must be BOB or USD", field: "currency") unless CURRENCIES.include?(code)

        code
      end

      def optional_seconds(value) = value.nil? ? nil : Core.seconds(value)

      def ref(reference) = { "reference" => reference.to_s }

      def webhook_call(headers, body, webhook)
        {
          "headers" => headers.to_h { |k, v| [k.to_s, v.to_s] },
          "body" => body.to_s.dup.force_encoding(Encoding::UTF_8),
          "key" => webhook.key,
          "value" => webhook.value
        }
      end
    end
  end
end
