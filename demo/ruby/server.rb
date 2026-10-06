# frozen_string_literal: true

# OpenHub QR demo service — Ruby (openhub-bo-qr gem).
#
# Serves demo/web/index.html and the demo API shared by the three language
# services. Environment: CLIENT_ID, CLIENT_SECRET, DEMO_PASSWORD, PORT (8103),
# PUBLIC_URL (optional; else derived from the Host header), DEMO_LINKS
# (optional "Python=https://...,TypeScript=...,Ruby=...").
#
#   ruby -I../../bindings/ruby/openhub-bo-core/lib -I../../bindings/ruby/openhub-bo-qr/lib server.rb

require "bigdecimal"
require "json"
require "openssl"
require "securerandom"
require "time"
require "webrick"
require "openhub_bo/qr"

LANGUAGE = "Ruby"
MAX_AMOUNT = BigDecimal("10")
INDEX = File.binread(File.expand_path("../web/index.html", __dir__))
PASSWORD = ENV.fetch("DEMO_PASSWORD")
WEBHOOK_SECRET = SecureRandom.urlsafe_base64(32)
Core = OpenhubBo::Core
QR = OpenhubBo::Qr::QrClient.new(Core::Session.new(ENV.fetch("CLIENT_ID"), ENV.fetch("CLIENT_SECRET")))
EVENTS = []
EVENTS_LOCK = Mutex.new

def links
  ENV.fetch("DEMO_LINKS", "").split(",").filter_map do |part|
    language, url = part.split("=", 2)
    { language: language, url: url } if url
  end
end

# OpenHub stores the merchant reference as a 32-bit integer.
def new_reference = ((Time.now.to_f * 1000).to_i % 2_147_483_647).to_s

def decimal_s(value) = value&.to_s("F")

def qr_json(qr, webhook_url)
  {
    kind: qr.kind, reference: qr.reference, merchantReference: qr.merchant_reference,
    status: qr.status, rawStatus: qr.raw_status, amount: decimal_s(qr.amount), currency: qr.currency,
    expiresAt: qr.expires_at&.iso8601, qrDataUri: "data:image/png;base64,#{qr.qr_image_base64}",
    webhookUrl: webhook_url
  }
end

def status_json(st)
  {
    kind: st.kind, reference: st.reference, status: st.status, rawStatus: st.raw_status,
    message: st.message, amount: decimal_s(st.amount),
    payer: st.payer&.name, payerBank: st.payer_bank&.bank_name
  }
end

def reply(res, status, payload, type = "application/json")
  res.status = status
  res["Content-Type"] = type
  res["Cache-Control"] = "no-store"
  res.body = type == "application/json" ? JSON.generate(payload) : payload
end

def authorized?(req, res)
  expected = "Basic #{["demo:#{PASSWORD}"].pack('m0')}"
  given = req["Authorization"].to_s
  return true if given.bytesize == expected.bytesize && OpenSSL.fixed_length_secure_compare(given, expected)

  res.status = 401
  res["WWW-Authenticate"] = 'Basic realm="openhub-demo"'
  false
end

def call(res)
  reply(res, 200, yield)
rescue Core::ValidationError => e
  reply(res, 400, { error: e.message, field: e.field })
rescue Core::ApiError => e
  reply(res, 422, { error: e.message, code: e.code, retryable: e.retryable? })
rescue Core::Error, ArgumentError, JSON::ParserError => e
  reply(res, 422, { error: "#{e.class.name}: #{e.message}" })
end

def generate(req, form)
  amount = begin
    BigDecimal(form.fetch("amount", "").to_s)
  rescue ArgumentError
    raise Core::ValidationError.new("must be a decimal number", field: "amount")
  end
  raise Core::ValidationError.new("the demo allows at most Bs #{MAX_AMOUNT.to_i}", field: "amount") if amount > MAX_AMOUNT

  kind = form["kind"] == "mld" ? :mld : :simple
  public_url = ENV["PUBLIC_URL"] || "https://#{req['Host'] || 'localhost'}"
  webhook_url = "#{public_url}/webhooks/qr/#{kind}"
  qr = QR.generate_qr(
    amount: amount, description: form.fetch("description", "Prueba demo").to_s,
    reference: new_reference, establishment_id: 1, establishment_name: "Demo openhub bo",
    expires_in: Integer(form.fetch("expiresIn", 600)), kind: kind,
    webhook: Core::Webhook.new(url: webhook_url, value: WEBHOOK_SECRET)
  )
  qr_json(qr, webhook_url)
end

def webhook(req, res, kind)
  event = { receivedAt: Time.now.utc.iso8601(3), kind: kind }
  status = 200
  begin
    headers = req.header.transform_values { |v| v.join(", ") }
    n = OpenhubBo::Qr.parse_webhook(headers, req.body.to_s, webhook: Core::Webhook.new(url: "", value: WEBHOOK_SECRET))
    event.merge!(reference: n.reference, status: n.status, amount: decimal_s(n.amount))
    event[:confirmedStatus] = QR.get_qr_status(n.reference, kind: kind).status
  rescue Core::WebhookAuthError => e
    status = 401
    event[:error] = "rechazado: #{e.message}"
  rescue Core::Error => e
    event[:error] = "#{e.class.name}: #{e.message} · body=#{req.body.to_s[0, 300]}"
  end
  EVENTS_LOCK.synchronize do
    EVENTS << event
    EVENTS.shift while EVENTS.size > 50
  end
  warn "webhook #{kind} -> #{status} #{event}"
  reply(res, status, { ok: status == 200 })
end

server = WEBrick::HTTPServer.new(
  Port: Integer(ENV.fetch("PORT", 8103)), BindAddress: "0.0.0.0",
  AccessLog: [], Logger: WEBrick::Log.new($stderr, WEBrick::Log::WARN)
)

server.mount_proc("/") do |req, res|
  path = req.path
  if req.request_method == "GET" && path == "/health"
    reply(res, 200, { ok: true, language: LANGUAGE })
  elsif req.request_method == "POST" && (m = %r{\A/webhooks/qr/(simple|mld)\z}.match(path))
    webhook(req, res, m[1].to_sym)
  elsif !authorized?(req, res)
    nil
  elsif req.request_method == "GET" && path == "/"
    reply(res, 200, INDEX, "text/html; charset=utf-8")
  elsif req.request_method == "GET" && path == "/api/info"
    reply(res, 200, { language: LANGUAGE, library: "openhub-bo-qr (gem)",
                      version: OpenhubBo::Qr::Native.version, environment: "sandbox", links: links })
  elsif req.request_method == "GET" && path == "/api/events"
    reply(res, 200, EVENTS_LOCK.synchronize { EVENTS.reverse })
  elsif req.request_method == "POST" && path == "/api/qr"
    call(res) { generate(req, JSON.parse(req.body || "{}")) }
  elsif req.request_method == "GET" && (m = %r{\A/api/qr/(simple|mld)/(\d+)\z}.match(path))
    call(res) { status_json(QR.get_qr_status(m[2], kind: m[1].to_sym)) }
  elsif req.request_method == "POST" && (m = %r{\A/api/qr/simple/(\d+)/cancel\z}.match(path))
    call(res) { status_json(QR.cancel_qr(m[1])) }
  else
    reply(res, 404, { error: "not found" })
  end
end

trap("INT") { server.shutdown }
trap("TERM") { server.shutdown }
warn "#{LANGUAGE} demo on :#{server.config[:Port]}"
server.start
