# frozen_string_literal: true

require_relative "test_helper"

class TestPayouts < Minitest::Test
  Core = OpenhubBo::Core
  Payouts = OpenhubBo::Payouts
  TOKEN = "tok_0123456789abcdef"
  PROCESS_ID = "66ec5b3d-61ea-4254-b366-7104545aa3c6"

  def setup
    @gateway = Core::Testing::MockGateway.new
    @client = Payouts::PayoutsClient.new(@gateway.session)
  end

  def stub(method, path, file, status: 200)
    @gateway.stub(method, path, status: status, body: fixture("payouts/#{file}"))
  end

  def last = @gateway.requests.last

  def transfer(id = "001002")
    Payouts::BatchTransfer.new(transaction_id: id, amount: "50.00", source_account: "484811311404044",
                               destination_account: "1311404044", bank_code: "1018", branch_city: "LPZ",
                               description: "Pago proveedor", recipient_document_id: "5452452",
                               recipient_name: "PROVEEDOR SRL", date: "2026-10-06")
  end

  def test_scan_pay_and_status
    stub("POST", %r{/payout/sync/v3/qr/scan\z}, "sandbox_scan_response.json")
    stub("POST", %r{/payout/sync/v3/qr/confirm\z}, "pay_response.json")
    stub("GET", %r{/payout/sync/v3/qr/status/547250827000000004\z}, "payment_status_response.json")

    scanned = @client.scan_qr("Wg74o8sx")
    assert_equal 40, last["timeout"]
    assert_equal ["547261006000003099", BigDecimal("1"), false], [scanned.reference, scanned.amount, scanned.open_amount?]

    payout = @client.pay_qr(scanned, source_account: "7010123451", transaction_id: "PAY-1")
    assert_equal 90, last["timeout"]
    assert_includes last["body"], '"importe":0.00' # docs: 0.00 for fixed-amount QRs
    refute_includes JSON.parse(last["body"]).keys, "glosa"
    assert_equal [:paid, BigDecimal("100.50")], [payout.status, payout.amount]
    assert_equal "La transacción fue aprobada.", @client.get_payout("547250827000000004").message
  end

  def test_open_amount_rules_and_unconfirmed
    open_qr = Payouts::ScannedQr.from_h(JSON.parse(fixture("payouts/scan_open_amount_response.json"))["data"].then do |d|
      { "reference" => d["numeroReferencia"], "amount" => "0", "currency" => "BOB", "description" => nil,
        "recipient" => {}, "expires_on" => nil }
    end)
    assert_predicate open_qr, :open_amount?
    assert_equal "amount", assert_raises(Core::ValidationError) { @client.pay_qr(open_qr, source_account: "7010123451", transaction_id: "P") }.field
    assert_equal "description", assert_raises(Core::ValidationError) { @client.pay_qr(open_qr, source_account: "7010123451", transaction_id: "P", amount: "5") }.field

    stub("POST", %r{/confirm\z}, "pay_unconfirmed_response.json")
    error = assert_raises(Core::AmbiguousOutcomeError) do
      @client.pay_qr(open_qr, source_account: "7010123451", transaction_id: "P", amount: "5", description: "Pago")
    end
    assert_equal ["payouts.pay", "96", false], [error.operation, error.code, error.retryable?]
  end

  def test_batches
    stub("POST", %r{/payout/async/v3/lote/autorizar\z}, "batch_authorize_response.json")
    stub("GET", %r{/payout/async/v3/lote/estado/#{PROCESS_ID}\z}, "batch_status_response.json")
    stub("POST", %r{/payout/async/v3/bancos\z}, "sandbox_banks_response.json")

    auth = @client.authorize_batch("455544", [transfer], webhook_url: "https://shop.example/ach", webhook_token: TOKEN,
                                                         process_id: PROCESS_ID)
    assert_equal "455544", last["headers"]["branchCode"]
    assert_equal "https://shop.example/ach?token=#{TOKEN}", JSON.parse(last["body"])["webhookUrl"]
    assert_equal [%i[pending error], ["001002"]], [auth.transfers.map(&:status), auth.rejected.map(&:transaction_id).push("001002").last(1)]

    status = @client.get_batch_status("455544", PROCESS_ID, batch_number: "2601191045")
    assert_match(/\?nroLote=2601191045\z/, last["url"])
    assert_equal [:paid, BigDecimal("85.00")], [status.transfers[0].status, status.transfers[0].amount]
    assert_equal "1005", @client.list_banks("455544")[0].code

    assert_match(/\A\h{8}-/, Payouts::Ops::AUTHORIZE.native.invoke("batch.authorize.build", {
      "config" => { "client_id" => "c", "client_secret" => "s" },
      "token" => { "access_token" => "t", "token_type" => "x", "expires_at" => 0, "scope" => nil },
      "input" => { "branch_code" => "1", "process_id" => SecureRandom.uuid, "webhook_url" => "https://a.bo",
                   "webhook_token" => TOKEN, "transfers" => [transfer.to_wire] }
    }).then { JSON.parse(_1["body"])["processId"] })
  end

  def test_batch_validation_and_webhook
    assert_equal "transfers.transaction_id", assert_raises(Core::ValidationError) {
      @client.authorize_batch("1", [transfer, transfer], webhook_url: "https://a.bo", webhook_token: TOKEN)
    }.field
    assert_equal "webhook_token", assert_raises(Core::ValidationError) {
      @client.authorize_batch("1", [transfer], webhook_url: "https://a.bo", webhook_token: "short")
    }.field

    n = Payouts.parse_batch_webhook(TOKEN, fixture("payouts/batch_webhook.json"), expected_token: TOKEN)
    assert_equal [:paid, "2601191045"], [n.transfer.status, n.batch_number]
    assert_equal "EXITOSO", n.ack["codigoRespuesta"]
    assert_equal "FALLIDO", n.ack(processed: false, detail: "dup")["codigoRespuesta"]
    assert_raises(Core::WebhookAuthError) { Payouts.parse_batch_webhook("wrong", "{}", expected_token: TOKEN) }
  end
end
