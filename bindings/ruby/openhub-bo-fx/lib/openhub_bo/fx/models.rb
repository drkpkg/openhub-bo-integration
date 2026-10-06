# frozen_string_literal: true

module OpenhubBo
  module Fx
    CURRENCIES = %w[BOB USD].freeze
    VIRTUAL_ASSETS = %i[usdt usdc bk].freeze

    # `glosa` in the `branch_code|branch_name|category|description` format the
    # FX APIs require (category is e.g. an MCC like "7011" or "MISCELANEAS").
    Glosa = Data.define(:branch_code, :branch_name, :category, :description) do
      def to_wire = [branch_code, branch_name, category, description].join("|")
    end

    FxQr = Core.model(
      :reference, :merchant_reference, :status, :raw_status, :detail, :amount, :currency, :converted_amount,
      :converted_currency, :exchange_rate, :expires_at, :image_mime, :image_base64,
      casts: { status: :symbol, amount: :decimal, converted_amount: :decimal, exchange_rate: :decimal }
    ) do
      def image = image_base64.unpack1("m")

      # Ready for <img src="..."> (PNG for PIX/Koibanx, JPEG for Binance).
      def data_uri = "data:#{image_mime};base64,#{image_base64}"

      def inspect = "#<#{self.class.name} reference=#{reference.inspect} status=#{status.inspect}>"
    end

    FxStatusInfo = Core.model(
      :reference, :status, :raw_status, :detail, :amount, :currency, :converted_amount, :converted_currency,
      :exchange_rate, :reversal, :requested_at,
      casts: { status: :symbol, amount: :decimal, converted_amount: :decimal, exchange_rate: :decimal }
    )

    # `ack` is the JSON body ATC requires as the webhook answer (HTTP 200).
    BinanceNotification = Core.model(
      :reference, :status, :raw_status, :amount, :currency, :transaction_at, :payer_name, :payer_document_id, :ack,
      casts: { status: :symbol, amount: :decimal }
    )

    # PIX / Koibanx payloads are undocumented: best-effort fields plus `payload`.
    FxNotification = Core.model(
      :reference, :status, :raw_status, :amount, :currency, :payload,
      casts: { status: :symbol, amount: :decimal }
    )
  end
end
