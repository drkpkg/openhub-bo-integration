# frozen_string_literal: true

module OpenhubBo
  module Payouts
    # Pay third-party QRs and send ACH transfer batches from your ATC account.
    # **Moves real money in production.**
    #
    # QR flow: scan_qr (show the recipient to the user) -> pay_qr -> get_payout.
    # pay_qr raises Core::AmbiguousOutcomeError when OpenHub cannot confirm the
    # result (codes 94/96) or the response is lost: query get_payout first.
    class PayoutsClient
      attr_reader :session

      def initialize(session)
        @session = session
      end

      # Decodes a QR from its *text content* (what a QR reader returns).
      def scan_qr(qr_text) = session.execute(Ops::SCAN, { "qr_text" => qr_text })

      # `amount` only for open-amount QRs; `description` only when the QR has
      # none; `transaction_id` is your unique id (at most 32 characters).
      def pay_qr(scanned, source_account:, transaction_id:, amount: nil, description: nil)
        session.execute(Ops::PAY, {
          "scanned" => scanned.to_wire, "source_account" => source_account.to_s,
          "transaction_id" => transaction_id, "amount" => amount.nil? ? nil : Core.amount(amount),
          "description" => description
        })
      end

      def get_payout(reference) = session.execute(Ops::STATUS, { "reference" => reference.to_s })

      # Submits a batch. ATC POSTs one notification per transfer to
      # `webhook_url?token=<webhook_token>` (see Payouts.parse_batch_webhook).
      # Keep `process_id` (a new UUID by default) to query the batch.
      def authorize_batch(branch_code, transfers, webhook_url:, webhook_token:, process_id: SecureRandom.uuid)
        session.execute(Ops::AUTHORIZE, {
          "branch_code" => branch_code.to_s, "process_id" => process_id, "webhook_url" => webhook_url,
          "webhook_token" => webhook_token, "transfers" => transfers.map(&:to_wire)
        })
      end

      # Give exactly one of `batch_number` or `transaction_id`.
      def get_batch_status(branch_code, process_id, batch_number: nil, transaction_id: nil)
        session.execute(Ops::BATCH_STATUS, {
          "branch_code" => branch_code.to_s, "process_id" => process_id,
          "batch_number" => batch_number, "transaction_id" => transaction_id
        })
      end

      def list_banks(branch_code) = session.execute(Ops::BANKS, { "branch_code" => branch_code.to_s })
    end

    module_function

    # Authenticates and parses one batch-transfer notification. `token` is the
    # `token` query parameter of the incoming request; answer with HTTP 200 and
    # `notification.ack` as JSON.
    def parse_batch_webhook(token, body, expected_token:)
      Ops::PARSE_WEBHOOK.call({
        "token" => token.to_s, "body" => body.to_s.dup.force_encoding(Encoding::UTF_8),
        "expected_token" => expected_token
      })
    end
  end
end
