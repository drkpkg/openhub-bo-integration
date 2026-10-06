# frozen_string_literal: true

require_relative "test_helper"

class TestQr < Minitest::Test
  Core = OpenhubBo::Core
  Qr = OpenhubBo::Qr
  WEBHOOK = Core::Webhook.new(url: "https://dominio.com/qr/confirmed", value: "46bc-b2a2-ea12258c99ab")

  def setup
    @gateway = Core::Testing::MockGateway.new
    @client = Qr::QrClient.new(@gateway.session)
  end

  def args(**overrides)
    { amount: "10.50", description: "Pago de servicio", reference: "4024", establishment_id: 422_717,
      establishment_name: "Tienda Central", expires_in: 45, webhook: WEBHOOK }.merge(overrides)
  end

  def test_generate_matches_documented_contract
    @gateway.stub("POST", %r{/qr/simple/v2/generate\z}, status: 200, body: fixture("generate_response.json"))
    qr = @client.generate_qr(**args)
    request = @gateway.requests.last
    assert_equal JSON.parse(fixture("generate_request.json")), JSON.parse(request["body"])
    assert_equal @gateway.client_id, request["headers"]["client_id"]
    assert_equal "Bearer #{request['headers']['access_token']}", request["headers"]["Authorization"]

    assert_equal ["153980", :simple, "2320", :pending], [qr.reference, qr.kind, qr.merchant_reference, qr.status]
    assert_equal BigDecimal("10.5"), qr.amount
    assert_equal Time.new(2026, 3, 12, 18, 1, 52.304302r, "-04:00"), qr.expires_at
    assert qr.qr_png.start_with?("\x89PNG".b)
    assert qr.frozen?
  end

  def test_mld_uses_its_own_path
    @gateway.stub("POST", %r{/qr/mld/v2/generate\z}, status: 200, body: fixture("generate_response.json"))
    assert_equal :mld, @client.generate_qr(**args(kind: :mld)).kind
    assert_match %r{/qr/mld/v2/generate\z}, @gateway.requests.last["url"]
  end

  def test_status_and_cancel
    @gateway.stub("GET", %r{/qr/simple/v2/verify/200393\z}, status: 200, body: fixture("verify_pending_response.json"))
    @gateway.stub("POST", %r{/qr/simple/v2/cancel/11195853\z}, status: 200, body: fixture("cancel_response.json"))
    @gateway.stub("POST", %r{/qr/simple/v2/cancel/1\z}, status: 409, body: fixture("cancel_invalid_state_response.json"))

    status = @client.get_qr_status("200393")
    assert_equal [:pending, :simple, BigDecimal("10.5"), nil, nil],
                 [status.status, status.kind, status.amount, status.payer, status.payer_bank]
    assert_equal :cancelled, @client.cancel_qr("11195853").status

    error = assert_raises(Core::ApiError) { @client.cancel_qr("1") }
    assert_equal [409, "ESTADO_INVALIDO", false], [error.status, error.code, error.retryable?]
  end

  def test_unknown_keys_from_core_are_ignored
    data = JSON.parse(fixture("generate_response.json"))["data"]
    model = Qr::Payer.from_h({ "name" => "Ana", "new_field" => 1 })
    assert_equal "Ana", model.name
    refute_nil data
  end

  def test_not_found
    @gateway.stub("GET", %r{/verify/9\z}, status: 404, body: fixture("error_not_found_response.json"))
    error = assert_raises(Core::ApiError) { @client.get_qr_status("9") }
    assert_equal "TRANSACCION_NO_ENCONTRADA", error.code
  end

  def test_validation_happens_before_network
    { { amount: "10.505" } => "amount", { reference: "ord-42" } => "reference",
      { description: " " } => "description", { expires_in: 0 } => "expires_in" }.each do |override, field|
      error = assert_raises(Core::ValidationError) { @client.generate_qr(**args(**override)) }
      assert_equal field, error.field
    end
    assert_raises(TypeError) { @client.generate_qr(**args(amount: 10.5)) }
    assert_raises(Core::ValidationError) { @client.generate_qr(**args(kind: :pix)) }
    assert(@gateway.requests.none? { _1["url"].end_with?("/generate") })
  end

  def test_lost_generate_response_is_ambiguous
    @gateway.route("POST", %r{/generate\z}) { raise Core::TransportError.new("read timeout", maybe_sent: true) }
    error = assert_raises(Core::AmbiguousOutcomeError) { @client.generate_qr(**args) }
    assert_equal "qr.generate", error.operation
  end

  def test_webhooks
    notification = Qr.parse_webhook({ "X-Api-Key" => WEBHOOK.value }, fixture("webhook_payment.json"), webhook: WEBHOOK)
    assert_equal [true, :paid, "233324", BigDecimal("10.5")],
                 [notification.success, notification.status, notification.reference, notification.amount]
    assert_equal "12345678", notification.payer.document_id
    assert_equal Time.new(2026, 5, 26, 14, 35, 20, "-04:00"), notification.transaction_at

    status_shaped = JSON.parse(fixture("verify_pending_response.json"))["data"].merge("estado" => "PAGADO")
    assert_equal "PAGADO", Qr.parse_webhook({ "x-api-key" => WEBHOOK.value }, JSON.generate(status_shaped), webhook: WEBHOOK).response_code

    assert_raises(Core::WebhookAuthError) { Qr.parse_webhook({ "x-api-key" => "nope" }, fixture("webhook_payment.json"), webhook: WEBHOOK) }
    assert_raises(Core::DecodeError) { Qr.parse_webhook({ "x-api-key" => WEBHOOK.value }, "<html>", webhook: WEBHOOK) }
  end

  def test_clients_share_the_session_token
    @gateway.stub("GET", %r{/verify/200393\z}, status: 200, body: fixture("verify_pending_response.json"))
    session = @gateway.session
    2.times { Qr::QrClient.new(session).get_qr_status("200393") }
    assert_equal 1, @gateway.token_requests
  end
end
