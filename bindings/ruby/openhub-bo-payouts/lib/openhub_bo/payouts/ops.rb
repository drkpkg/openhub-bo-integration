# frozen_string_literal: true

module OpenhubBo
  module Payouts
    # Operations declared against the native extension (checked at require time).
    module Ops
      NATIVE = Core::NativeBridge.new(Payouts::Native)
      SCAN = Core::Op.new(NATIVE, "payouts.scan") { ScannedQr.from_h(_1) }
      PAY = Core::Op.new(NATIVE, "payouts.pay") { Payout.from_h(_1) }
      STATUS = Core::Op.new(NATIVE, "payouts.status") { Payout.from_h(_1) }
      AUTHORIZE = Core::Op.new(NATIVE, "batch.authorize") { BatchAuthorization.from_h(_1) }
      BATCH_STATUS = Core::Op.new(NATIVE, "batch.status") { BatchStatus.from_h(_1) }
      BANKS = Core::Op.new(NATIVE, "batch.banks") { |rows| rows.map { Bank.from_h(_1) } }
      PARSE_WEBHOOK = Core::Handler.new(NATIVE, "batch.webhook.parse") { BatchNotification.from_h(_1) }
    end
  end
end
