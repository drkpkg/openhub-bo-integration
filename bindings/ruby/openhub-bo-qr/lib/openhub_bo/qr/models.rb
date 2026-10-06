# frozen_string_literal: true

module OpenhubBo
  module Qr
    KINDS = %i[simple mld].freeze

    Payer = Core.model(:name, :account_number, :document_id)
    PayerBank = Core.model(:bank_code, :bank_name, :ach_order_number, :transaction_date)

    # `kind` (:simple / :mld) travels with the reference: Simple and MLD
    # references live in separate spaces, so keep both to query the QR later.
    GeneratedQr = Core.model(
      :reference, :kind, :merchant_reference, :status, :raw_status, :expires_at, :currency, :amount, :qr_image_base64,
      casts: { kind: :symbol, status: :symbol, expires_at: :time, amount: :decimal }
    ) do
      # PNG bytes of the QR.
      def qr_png = qr_image_base64.unpack1("m")

      # Ready for <img src="...">.
      def qr_data_uri = "data:image/png;base64,#{qr_image_base64}"

      def inspect = "#<#{self.class.name} reference=#{reference.inspect} status=#{status.inspect} amount=#{amount.to_s('F')}>"
    end

    QrStatusInfo = Core.model(
      :reference, :kind, :merchant_reference, :status, :raw_status, :message, :amount, :currency, :payer, :payer_bank,
      casts: { kind: :symbol, status: :symbol, amount: :decimal, payer: Payer, payer_bank: PayerBank }
    )

    PaymentNotification = Core.model(
      :reference, :amount, :currency, :status, :success, :response_code, :response_detail, :transaction_at,
      :payer, :payer_bank,
      casts: { amount: :decimal, status: :symbol, transaction_at: :time, payer: Payer, payer_bank: PayerBank }
    )
  end
end
