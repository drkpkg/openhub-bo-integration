# frozen_string_literal: true

module OpenhubBo
  module Payouts
    Recipient = Core.model(:account, :document_id, :holder, :bank_code, :bank_name) do
      def to_wire = to_h.transform_keys(&:to_s)
    end

    # A decoded QR: show it to the user, then pass it to PayoutsClient#pay_qr.
    ScannedQr = Core.model(
      :reference, :amount, :currency, :description, :recipient, :expires_on,
      casts: { amount: :decimal, recipient: Recipient }
    ) do
      # Open-amount QRs (amount zero): the payer chooses the amount.
      def open_amount? = amount.zero?

      def to_wire
        {
          "reference" => reference, "amount" => amount.to_s("F"), "currency" => currency,
          "description" => description, "recipient" => recipient.to_wire, "expires_on" => expires_on
        }
      end
    end

    Payout = Core.model(
      :reference, :transaction_id, :transaction_at, :status, :raw_status, :message, :amount, :currency,
      :description, :source_account, :source_holder, :recipient, :ach_order_number,
      casts: { status: :symbol, amount: :decimal, recipient: Recipient }
    )

    # One ACH transfer of a batch. `transaction_id`: 3 to 14 characters, unique
    # in the batch. `branch_city`: CBB, COB, LPZ, ORU, POT, SCZ, SUC, TJA, TRI.
    BatchTransfer = Data.define(
      :transaction_id, :amount, :source_account, :destination_account, :bank_code, :branch_city, :description,
      :recipient_document_id, :recipient_name, :currency, :date
    ) do
      def initialize(currency: "BOB", date: Date.today, **rest) = super

      def to_wire
        {
          "transaction_id" => transaction_id.to_s, "amount" => Core.amount(amount), "date" => Core.iso_date(date),
          "source_account" => source_account.to_s, "destination_account" => destination_account.to_s,
          "bank_code" => bank_code.to_s, "branch_city" => branch_city.to_s, "description" => description,
          "recipient_document_id" => recipient_document_id.to_s, "recipient_name" => recipient_name,
          "currency" => currency
        }
      end
    end

    BatchTransferResult = Core.model(:transaction_id, :status, :raw_status, :reference, :message,
                                     casts: { status: :symbol })
    BatchAuthorization = Core.model(:batch_number, :process_id, :transfers, casts: { transfers: [BatchTransferResult] }) do
      # Transfers rejected up front (e.g. bank not enabled).
      def rejected = transfers.select { _1.status == :error }
    end
    BatchTransferStatus = Core.model(
      :transaction_id, :reference, :status, :raw_status, :message, :source_account, :destination_account,
      :ach_number, :recipient_number, :recipient_document_id, :recipient_name, :transaction_at, :bank_code,
      :bank_name, :amount, :currency,
      casts: { status: :symbol, amount: :decimal }
    )
    BatchStatus = Core.model(:batch_number, :transfers, casts: { transfers: [BatchTransferStatus] })
    Bank = Core.model(:code, :name)

    BatchNotification = Core.model(:batch_number, :transfer, casts: { transfer: BatchTransferStatus }) do
      # JSON body to answer the webhook with (HTTP 200).
      def ack(processed: true, detail: nil)
        {
          "nroLote" => batch_number, "numeroReferencia" => transfer.reference,
          "codigoRespuesta" => processed ? "EXITOSO" : "FALLIDO", "detalleRespuesta" => detail
        }
      end
    end
  end
end
