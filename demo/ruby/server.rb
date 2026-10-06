# frozen_string_literal: true

# OpenHub QR demo service — Ruby (openhub-bo-qr gem).
#
# Serves its demo app (demo/apps/cobros) and the demo API shared by the three language
# services. Environment: CLIENT_ID, CLIENT_SECRET, DEMO_PASSWORD, PORT (8103),
# PUBLIC_URL (optional; else derived from the Host header), DEMO_LINKS
# (optional "Python=https://...,TypeScript=...,Ruby=...").
#
#   ruby -I../../bindings/ruby/openhub-bo-core/lib -I../../bindings/ruby/openhub-bo-qr/lib server.rb

require "bigdecimal"
require "json"
require "net/http"
require "openssl"
require "securerandom"
require "set"
require "time"
require "uri"
require "webrick"
require "openhub_bo/qr"

LANGUAGE = "Ruby"
APP = "cobros"
COOKIE = "demo_#{APP}" # per app: browsers share cookies across localhost ports
MAX_AMOUNT = BigDecimal("10")
INDEX = File.binread(File.expand_path("../apps/#{APP}/index.html", __dir__))
LOGIN = File.binread(File.expand_path("../web/login.html", __dir__))
SESSIONS = Set.new
PASSWORD = ENV.fetch("DEMO_PASSWORD")
WEBHOOK_SECRET = SecureRandom.urlsafe_base64(32)
Core = OpenhubBo::Core
QR = OpenhubBo::Qr::QrClient.new(Core::Session.new(ENV.fetch("CLIENT_ID"), ENV.fetch("CLIENT_SECRET")))
EVENTS = []
EVENTS_LOCK = Mutex.new
PAYMENTS = {} # "kind:reference" -> record shown by the app

# Keeps the app's payment list in sync with what OpenHub reports.
def track(kind, reference, **fields)
  EVENTS_LOCK.synchronize do
    record = PAYMENTS["#{kind}:#{reference}"]
    record&.merge!(fields, updatedAt: Time.now.utc.iso8601(3))
  end
end

def links
  ENV.fetch("DEMO_LINKS", "").split(",").filter_map do |part|
    language, url = part.split("=", 2)
    { language: language, url: url } if url
  end
end

# OpenHub stores the merchant reference as a 32-bit integer.
def new_reference = ((Time.now.to_f * 1000).to_i % 2_147_483_647).to_s

def decimal_s(value) = value&.to_s("F")

# A paid notification shaped like ATC's (fixtures/openhub/webhook_payment.json).
# Fictitious payer; only used by the demo's "Simular pago" button.
def simulated_webhook(record)
  JSON.generate(
    detalleRespuesta: "Transacción procesada correctamente", codigoRespuesta: "SUCCESS",
    numeroReferencia: record[:reference], monto: Float(record[:amount]),
    fechaHoraTransaccion: Time.now.getlocal("-04:00").strftime("%Y-%m-%dT%H:%M:%S"), moneda: record[:currency],
    clienteOrigen: { ciCliente: "0000000", nombreCliente: "Cliente de prueba", numeroCuenta: "0000000000" },
    bancoOrigen: { codigoBanco: "000", nombreBanco: "Banco simulado", numeroOrdenAch: "SIM#{record[:reference]}" }
  )
end

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

def secure_equal?(a, b) = a.bytesize == b.bytesize && OpenSSL.fixed_length_secure_compare(a, b)

def redirect(res, location)
  res.status = 303
  res["Location"] = location
end

# Session cookie (browser) or HTTP Basic (scripts). Never triggers the browser's
# Basic-auth dialog: pages redirect to /login, APIs get 401.
def authorized?(req, res)
  return true if secure_equal?(req["Authorization"].to_s, "Basic #{["demo:#{PASSWORD}"].pack('m0')}")

  return true if req.cookies.any? { |c| c.name == COOKIE && SESSIONS.include?(c.value) }

  req.path == "/" ? redirect(res, "/login") : reply(res, 401, { error: "unauthorized" })
  false
end

def login(req, res)
  password = URI.decode_www_form(req.body.to_s).to_h.fetch("password", "")
  return redirect(res, "/login?error=1") unless secure_equal?(password, PASSWORD)

  token = SecureRandom.urlsafe_base64(24)
  SESSIONS << token
  secure = req["X-Forwarded-Proto"] == "https" ? "; Secure" : ""
  res["Set-Cookie"] = "#{COOKIE}=#{token}; Path=/; HttpOnly; SameSite=Lax#{secure}"
  redirect(res, "/")
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
  result = qr_json(qr, webhook_url)
  now = Time.now.utc.iso8601(3)
  EVENTS_LOCK.synchronize do
    PAYMENTS["#{qr.kind}:#{qr.reference}"] = {
      kind: qr.kind, reference: qr.reference, merchantReference: qr.merchant_reference,
      label: (form["label"] || form["description"]).to_s, amount: decimal_s(qr.amount), currency: qr.currency,
      status: qr.status, expiresAt: result[:expiresAt], webhookUrl: webhook_url, createdAt: now, updatedAt: now
    }
  end
  result
end

def status_and_track(kind, reference)
  record = EVENTS_LOCK.synchronize { PAYMENTS["#{kind}:#{reference}"]&.dup }
  if record&.dig(:simulated) # ATC still reports it pending: keep the simulated result
    return { kind: kind, reference: reference, status: record[:status], rawStatus: "PAGADO (simulado)",
             message: nil, amount: record[:amount], payer: record[:payer], payerBank: record[:payerBank],
             simulated: true }
  end

  result = status_json(QR.get_qr_status(reference, kind: kind))
  track(kind, reference, status: result[:status], payer: result[:payer], payerBank: result[:payerBank])
  result
end

def cancel_and_track(reference)
  result = status_json(QR.cancel_qr(reference))
  track(:simple, reference, status: result[:status])
  result
end

# Sends ATC's paid notification to this service's public webhook URL, through the
# tunnel, with the QR's secret header: same path as a real payment.
def simulate(kind, reference)
  record = EVENTS_LOCK.synchronize { PAYMENTS["#{kind}:#{reference}"]&.dup }
  raise ArgumentError, "unknown QR in this session" unless record
  raise ArgumentError, "only pending QRs can be paid (status: #{record[:status]})" unless record[:status].to_s == "pending"

  uri = URI(record[:webhookUrl])
  response = begin
    Net::HTTP.start(uri.host, uri.port, use_ssl: uri.scheme == "https", open_timeout: 10, read_timeout: 20) do |http|
      http.post(uri.path, simulated_webhook(record),
                "Content-Type" => "application/json", "x-api-key" => WEBHOOK_SECRET, "X-Demo-Simulated" => "1")
    end
  rescue StandardError => e
    raise ArgumentError, "webhook no entregado: #{e.message}"
  end
  { webhookUrl: record[:webhookUrl], webhookStatus: response.code.to_i, **status_and_track(kind, reference) }
end

def webhook(req, res, kind)
  event = { receivedAt: Time.now.utc.iso8601(3), kind: kind }
  status = 200
  begin
    headers = req.header.transform_values { |v| v.join(", ") }
    n = OpenhubBo::Qr.parse_webhook(headers, req.body.to_s, webhook: Core::Webhook.new(url: "", value: WEBHOOK_SECRET))
    event.merge!(reference: n.reference, status: n.status, amount: decimal_s(n.amount))
    payer = { payer: n.payer&.name, payerBank: n.payer_bank&.bank_name }
    if req["X-Demo-Simulated"] == "1"
      # ATC knows nothing about a simulated payment: don't ask it to confirm.
      event.merge!(simulated: true, confirmedStatus: n.status)
      track(kind, n.reference, status: n.status, viaWebhook: true, simulated: true, **payer)
    else
      event[:confirmedStatus] = QR.get_qr_status(n.reference, kind: kind).status
      track(kind, n.reference, status: event[:confirmedStatus], viaWebhook: true, **payer)
    end
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
  elsif path == "/login"
    req.request_method == "POST" ? login(req, res) : reply(res, 200, LOGIN, "text/html; charset=utf-8")
  elsif !authorized?(req, res)
    nil
  elsif req.request_method == "GET" && path == "/"
    reply(res, 200, INDEX, "text/html; charset=utf-8")
  elsif req.request_method == "GET" && path == "/api/info"
    reply(res, 200, { language: LANGUAGE, app: APP, library: "openhub-bo-qr (gem)",
                      version: OpenhubBo::Qr::Native.version, environment: "sandbox", links: links })
  elsif req.request_method == "GET" && path == "/api/payments"
    reply(res, 200, EVENTS_LOCK.synchronize { PAYMENTS.values.sort_by { |r| r[:createdAt] }.reverse })
  elsif req.request_method == "GET" && path == "/api/events"
    reply(res, 200, EVENTS_LOCK.synchronize { EVENTS.reverse })
  elsif req.request_method == "POST" && path == "/api/qr"
    call(res) { generate(req, JSON.parse(req.body || "{}")) }
  elsif req.request_method == "GET" && (m = %r{\A/api/qr/(simple|mld)/(\d+)\z}.match(path))
    call(res) { status_and_track(m[1].to_sym, m[2]) }
  elsif req.request_method == "POST" && (m = %r{\A/api/qr/simple/(\d+)/cancel\z}.match(path))
    call(res) { cancel_and_track(m[1]) }
  elsif req.request_method == "POST" && (m = %r{\A/api/qr/(simple|mld)/(\d+)/simulate\z}.match(path))
    call(res) { simulate(m[1].to_sym, m[2]) }
  else
    reply(res, 404, { error: "not found" })
  end
end

trap("INT") { server.shutdown }
trap("TERM") { server.shutdown }
warn "#{LANGUAGE} demo on :#{server.config[:Port]}"
server.start
