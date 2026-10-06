# frozen_string_literal: true

require_relative "test_helper"

class TestFx < Minitest::Test
  Core = OpenhubBo::Core
  Fx = OpenhubBo::Fx
  WEBHOOK = Core::Webhook.new(url: "https://shop.example/hook", value: "s3cret")
  GLOSA = Fx::Glosa.new(branch_code: "311113", branch_name: "Compras QR Calacoto La paz", category: "7011",
                        description: "Compra por Web QR")

  def setup
    @gateway = Core::Testing::MockGateway.new
    session = @gateway.session
    @pix = Fx::PixClient.new(session)
    @crypto = Fx::VirtualAssetsClient.new(session)
    @binance = Fx::BinanceClient.new(session)
  end

  def pix_args(**overrides)
    { amount: "145.00", glosa: GLOSA, reference: "311113", payer_cpf: "12345678901",
      payer_phone: "+5511999999999", webhook: WEBHOOK, expires_in: 120 }.merge(overrides)
  end

  def body = JSON.parse(@gateway.requests.last["body"])

  def test_pix_generate_status_and_cancel
    @gateway.stub("POST", %r{/qr/pix/v2/generar\z}, status: 200, body: fixture("fx/pix_generate_response.json"))
    @gateway.stub("GET", %r{/qr/pix/v2/verifica/6780\z}, status: 200, body: fixture("fx/pix_verify_response.json"))
    @gateway.stub("GET", %r{/qr/pix/v2/cancela/6780\z}, status: 200, body: fixture("fx/pix_cancel_response.json"))

    qr = @pix.generate_qr(**pix_args)
    assert_equal "311113|Compras QR Calacoto La paz|7011|Compra por Web QR", body["glosa"]
    assert_equal ["00:02:00", "WEB", "+5511999999999"], body.values_at("tiempoQr", "canal", "telefono")
    assert_equal ["6780", :pending, BigDecimal("592.52"), "BRL", "image/png"],
                 [qr.reference, qr.status, qr.converted_amount, qr.converted_currency, qr.image_mime]
    assert_equal :cancelled, @pix.get_status("6780").status
    refunded = @pix.cancel("6780")
    assert_equal ["6780", BigDecimal("592.52")], [refunded.reference, refunded.converted_amount]
  end

  def test_virtual_assets_and_binance
    @gateway.stub("POST", %r{/qr/koibanx/v2/generar\z}, status: 200, body: fixture("fx/crypto_generate_response.json"))
    @gateway.stub("GET", %r{/qr/koibanx/v2/estado/200699\z}, status: 200, body: fixture("fx/crypto_status_response.json"))
    @gateway.stub("POST", %r{/qr/binance/v2/generar\z}, status: 200, body: fixture("fx/binance_generate_response.json"))
    @gateway.stub("GET", %r{/qr/binance/v2/verificar/11193577\z}, status: 200, body: fixture("fx/binance_status_response.json"))

    qr = @crypto.generate_qr(amount: "50", glosa: GLOSA, reference: "321", asset: :usdc, webhook: WEBHOOK)
    assert_equal ["UP", 180], body.values_at("activoVirtual", "tiempoVencimientoQR")
    assert_equal "USDC", qr.converted_currency
    assert_equal [:expired, "200699"], [@crypto.get_status("200699").status, @crypto.get_status("200699").reference]

    qr = @binance.generate_qr(amount: "0.01", glosa: GLOSA, reference: "200397", webhook: WEBHOOK, expires_in: 300)
    assert_equal "00:05:00", body["tiempoQr"]
    assert_equal ["image/jpeg", BigDecimal("0.00083046")], [qr.image_mime, qr.converted_amount]
    assert_equal :pending, @binance.get_status("11193577").status
  end

  def test_validation_mirrors_sandbox_rules
    { { glosa: "Pago" } => "glosa", { payer_phone: "+59171234567" } => "payer_phone",
      { payer_cpf: "123" } => "payer_cpf", { reference: "ord-1" } => "reference" }.each do |override, field|
      assert_equal field, assert_raises(Core::ValidationError) { @pix.generate_qr(**pix_args(**override)) }.field
    end
    crypto = { glosa: GLOSA, reference: "1", asset: :usdt, webhook: WEBHOOK }
    assert_equal "amount", assert_raises(Core::ValidationError) { @crypto.generate_qr(amount: "49.99", **crypto) }.field
    assert_equal "expires_in", assert_raises(Core::ValidationError) { @crypto.generate_qr(amount: "50", expires_in: 60, **crypto) }.field
    assert_equal "asset", assert_raises(Core::ValidationError) { @crypto.generate_qr(amount: "50", **crypto, asset: :doge) }.field
    assert_equal "reference", assert_raises(Core::ValidationError) {
      @binance.generate_qr(amount: "1", glosa: GLOSA, reference: "12345678901", webhook: WEBHOOK)
    }.field
    assert_raises(TypeError) { @pix.generate_qr(**pix_args(amount: 145.0)) }
    assert(@gateway.requests.none? { _1["url"].end_with?("/generar") })
  end

  def test_sandbox_errors
    @gateway.stub("POST", %r{/qr/pix/v2/generar\z}, status: 200, body: fixture("fx/sandbox_pix_merchant_disabled.json"))
    @gateway.stub("GET", %r{/qr/pix/v2/verifica/1\z}, status: 500, body: fixture("fx/sandbox_pix_not_found.json"))
    error = assert_raises(Core::ApiError) { @pix.generate_qr(**pix_args) }
    assert_equal ["GQ-00005", false], [error.code, error.retryable?]
    error = assert_raises(Core::ApiError) { @pix.get_status("1") }
    assert_equal [500, "EG-00001", false], [error.status, error.code, error.retryable?]
  end

  def test_webhooks
    n = Fx.parse_binance_webhook({ "x-api-key" => "s3cret" }, fixture("fx/binance_webhook.json"), webhook: WEBHOOK)
    assert_equal ["4221", :paid, BigDecimal("1.00")], [n.reference, n.status, n.amount]
    assert_equal({ "numeroReferencia" => "4221", "codigoRespuesta" => "00", "detalleRespuesta" => nil }, n.ack)
    generic = Fx.parse_fx_webhook({ "x-api-key" => "s3cret" }, '{"numeroReferencia":6780,"codigoRespuesta":"PAID","extra":1}', webhook: WEBHOOK)
    assert_equal ["6780", :paid, 1], [generic.reference, generic.status, generic.payload["extra"]]
    assert_raises(Core::WebhookAuthError) { Fx.parse_binance_webhook({}, fixture("fx/binance_webhook.json"), webhook: WEBHOOK) }
  end
end
