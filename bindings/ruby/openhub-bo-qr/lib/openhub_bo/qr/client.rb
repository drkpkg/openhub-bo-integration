# frozen_string_literal: true

module OpenhubBo
  module Qr
    # Collect payments with QR Simple or QR MLD-BCB.
    #
    #   session = OpenhubBo::Core::Session.new(ENV["CLIENT_ID"], ENV["CLIENT_SECRET"])
    #   qr = OpenhubBo::Qr::QrClient.new(session).generate_qr(amount: "10.50", ...)
    class QrClient
      attr_reader :session

      def initialize(session)
        @session = session
      end

      # `reference` must be numeric; `expires_in` is in seconds; OpenHub requires
      # a webhook. A lost response raises Core::AmbiguousOutcomeError (the QR
      # may exist).
      def generate_qr(amount:, description:, reference:, establishment_id:, establishment_name:, expires_in:,
                      webhook:, kind: :simple)
        session.execute(Ops::GENERATE, {
          "kind" => Ops.kind(kind),
          "description" => description,
          "amount" => Core.amount(amount),
          "reference" => reference.to_s,
          "expires_in" => Core.seconds(expires_in),
          "establishment_id" => establishment_id,
          "establishment_name" => establishment_name,
          "webhook" => webhook.to_wire
        })
      end

      # `reference` is GeneratedQr#reference (ATC's reference); pass the same
      # `kind` the QR was generated with (GeneratedQr#kind): Simple and MLD
      # references live in separate spaces.
      def get_qr_status(reference, kind: :simple)
        session.execute(Ops::VERIFY, { "kind" => Ops.kind(kind), "reference" => reference.to_s })
      end

      # Cancels a pending QR Simple. MLD-BCB QRs cannot be cancelled: OpenHub has
      # no cancel endpoint for them (the gateway answers 404); let them expire.
      # Non-pending QRs raise Core::ApiError with code "ESTADO_INVALIDO" (HTTP 409).
      def cancel_qr(reference)
        session.execute(Ops::CANCEL, { "reference" => reference.to_s })
      end
    end

    module_function

    # Authenticates and parses a payment notification sent by ATC. `webhook` is
    # the Core::Webhook given to generate_qr. ATC does not sign payloads:
    # confirm with get_qr_status (and check the amount) before releasing goods.
    def parse_webhook(headers, body, webhook:)
      Ops::PARSE_WEBHOOK.call({
        "headers" => headers.to_h { |k, v| [k.to_s, v.to_s] },
        "body" => body.to_s.dup.force_encoding(Encoding::UTF_8),
        "key" => webhook.key,
        "value" => webhook.value
      })
    end
  end
end
