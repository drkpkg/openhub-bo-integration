# frozen_string_literal: true

module OpenhubBo
  module Qr
    # Operations declared against the native extension (checked at require time).
    module Ops
      NATIVE = Core::NativeBridge.new(Qr::Native)
      GENERATE = Core::Op.new(NATIVE, "qr.generate") { GeneratedQr.from_h(_1) }
      VERIFY = Core::Op.new(NATIVE, "qr.verify") { QrStatusInfo.from_h(_1) }
      CANCEL = Core::Op.new(NATIVE, "qr.cancel") { QrStatusInfo.from_h(_1) }
      PARSE_WEBHOOK = Core::Handler.new(NATIVE, "qr.webhook.parse") { PaymentNotification.from_h(_1) }

      module_function

      def kind(value)
        sym = value.to_sym
        raise Core::ValidationError.new("must be one of #{KINDS.join(', ')}", field: "kind") unless KINDS.include?(sym)

        sym.to_s
      end
    end
  end
end
