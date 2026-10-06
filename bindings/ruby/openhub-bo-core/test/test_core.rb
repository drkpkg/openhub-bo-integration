# frozen_string_literal: true

require_relative "test_helper"
require "socket"

class TestCore < Minitest::Test
  include OpenhubBo::Core

  def setup
    @gateway = Testing::MockGateway.new
    @fake_native = NativeBridge.new(FakeNative)
    @write = Op.new(@fake_native, "fake.write")
  end

  def test_native_catalog_and_version
    assert_equal({ "idempotent" => true, "kind" => "operation", "name" => "token" }, NATIVE.operations["token"])
    assert_equal OpenhubBo::Core::VERSION, NATIVE.version
  end

  def test_protocol_mismatch_is_a_load_error
    old = Module.new do
      def self.protocol_version = 1
      def self.version = "0"
    end
    error = assert_raises(LoadError) { NativeBridge.new(old) }
    assert_match(/protocol 1/, error.message)
  end

  def test_unknown_operation_fails_at_declaration
    assert_raises(LoadError) { Op.new(NATIVE, "qr.nope") }
  end

  def test_missing_credentials_fail_at_construction
    error = assert_raises(ValidationError) { Session.new("client-id") }
    assert_equal "client_secret", error.field
    assert_raises(ValidationError) { Session.new("id", "secret", environment: :staging) }
  end

  def test_token_is_fetched_once_and_shared
    @gateway.route("POST", %r{/fake\z}) { Response.new(status: 200, body: "{}") }
    session = @gateway.session
    3.times { assert_equal({ "done" => true }, session.execute(@write_op = @write, {})) }
    assert_equal 1, @gateway.token_requests
    assert_equal "access_token", JSON.parse(@gateway.perform(token_request).body)["token_type"]
  end

  def test_rejected_token_is_refreshed_once
    @gateway.route("POST", %r{/fake\z}) { Response.new(status: 200, body: "{}") }
    session = @gateway.session
    session.execute(@write, {})
    @gateway.revoke_tokens
    assert_equal({ "done" => true }, session.execute(@write, {}))
    assert_equal 2, @gateway.token_requests
  end

  def test_bad_credentials
    error = assert_raises(AuthenticationError) { @gateway.session(client_secret: "wrong").execute(@write, {}) }
    assert_equal 401, error.status
  end

  def test_gateway_errors_are_retryable
    @gateway.stub("POST", %r{/fake\z}, status: 502, body: "Error forwarding call")
    error = assert_raises(ApiError) { @gateway.session.execute(@write, {}) }
    assert_equal 502, error.status
    assert_predicate error, :retryable?
  end

  def test_lost_response_on_non_idempotent_op_is_ambiguous
    @gateway.route("POST", %r{/fake\z}) { raise TransportError.new("read timeout", maybe_sent: true) }
    error = assert_raises(AmbiguousOutcomeError) { @gateway.session.execute(@write, {}) }
    assert_equal "fake.write", error.operation
    refute_predicate error, :retryable?
    refute_kind_of TransportError, error
  end

  def test_connection_refused_is_a_plain_transport_error
    @gateway.route("POST", %r{/fake\z}) { raise TransportError.new("refused", maybe_sent: false) }
    error = assert_raises(TransportError) { @gateway.session.execute(@write, {}) }
    assert_predicate error, :retryable?
  end

  def test_op_timeout_reaches_the_transport
    @gateway.route("POST", %r{/fake\z}) { Response.new(status: 200, body: "{}") }
    @gateway.session.execute(@write, {})
    assert_equal 90, @gateway.requests.last["timeout"]
    assert_nil @gateway.requests.first["timeout"] # token: no recommended timeout
  end

  def test_error_mapping
    api = OpenhubBo::Core.error_from_core(
      { "kind" => "api", "status" => 404, "message" => "m", "errors" => [{ "message" => "x", "code" => "TRANSACCION_NO_ENCONTRADA" }] }
    )
    assert_equal ["TRANSACCION_NO_ENCONTRADA", false], [api.code, api.retryable?]
    ambiguous = OpenhubBo::Core.error_from_core(
      { "kind" => "ambiguous", "message" => "no confirmado", "errors" => [{ "message" => "m", "code" => "96" }] },
      "payouts.pay.parse"
    )
    assert_equal ["payouts.pay", "96"], [ambiguous.operation, ambiguous.code]
    assert_kind_of CoreError, OpenhubBo::Core.error_from_core({ "kind" => "ffi", "message" => "bad" })
  end

  def test_native_errors_round_trip
    error = assert_raises(ValidationError) { NATIVE.invoke("token.build", { "config" => { "client_id" => "" }, "input" => { "now" => 0 } }) }
    assert_equal "client_id", error.field
  end

  def test_net_http_transport_classifies_failures
    closed_port = TCPServer.open("127.0.0.1", 0).then { |s| s.addr[1].tap { s.close } }
    refused = assert_raises(TransportError) { NetHttpTransport.new.perform(request("http://127.0.0.1:#{closed_port}/x")) }
    refute refused.maybe_sent

    silent = TCPServer.open("127.0.0.1", 0)
    Thread.new { silent.accept && sleep(2) }
    started = Process.clock_gettime(Process::CLOCK_MONOTONIC)
    lost = assert_raises(TransportError) do
      NetHttpTransport.new(timeout: 30).perform(request("http://127.0.0.1:#{silent.addr[1]}/x", timeout: 0.3))
    end
    assert lost.maybe_sent
    assert_operator Process.clock_gettime(Process::CLOCK_MONOTONIC) - started, :<, 2, "per-request timeout must win"
  ensure
    silent&.close
  end

  def test_value_helpers
    assert_raises(TypeError) { OpenhubBo::Core.amount(10.5) }
    assert_equal "10.5", OpenhubBo::Core.amount(BigDecimal("10.50"))
    assert_equal "7", OpenhubBo::Core.amount(7)
    assert_equal Time.new(2026, 5, 26, 14, 35, 20, "-04:00"), OpenhubBo::Core.local_time("2026-05-26T14:35:20")
    assert_equal Time.new(2026, 9, 17, 14, 43, 51, "-04:00"), OpenhubBo::Core.local_time("2026-09-17 14:43:51")
    assert PaymentStatus.final?(:paid)
    refute PaymentStatus.final?(:pending)
  end

  def test_webhook_secret_is_hidden
    webhook = Webhook.new(url: "https://a.bo/h", value: "s3cret")
    refute_includes webhook.inspect, "s3cret"
    assert_equal({ "url" => "https://a.bo/h", "key" => "x-api-key", "value" => "s3cret" }, webhook.to_wire)
  end

  private

  def token_request
    { "method" => "POST", "url" => "#{Testing::MockGateway::BASE_URL}/oauth-client-credentials/access-token",
      "headers" => { "Authorization" => "Basic #{['test-client:test-secret'].pack('m0')}" }, "body" => nil }
  end

  def request(url, timeout: nil)
    { "method" => "GET", "url" => url, "headers" => {}, "body" => nil, "timeout" => timeout }.compact
  end
end
