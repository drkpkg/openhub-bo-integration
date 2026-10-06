# frozen_string_literal: true

# End-to-end check against the real OpenHub sandbox: generate, query, cancel.
#
#   cd bindings/ruby && rake compile    # once
#   ruby -Iopenhub-bo-core/lib -Iopenhub-bo-qr/lib examples/sandbox_smoke.rb
#
# Reads CLIENT_ID / CLIENT_SECRET from the environment or the repo's .env.
# Creates a Bs 1.00 QR (valid 2 minutes) and cancels it. Never prints secrets.
require "bigdecimal"
require "openhub_bo/qr"

env_file = File.expand_path("../../../.env", __dir__)
if File.exist?(env_file)
  File.foreach(env_file) do |line|
    key, value = line.strip.split("=", 2)
    ENV[key] ||= value if key && value && !key.start_with?("#")
  end
end

Core = OpenhubBo::Core
session = Core::Session.new(ENV.fetch("CLIENT_ID"), ENV.fetch("CLIENT_SECRET"))
client = OpenhubBo::Qr::QrClient.new(session)
webhook = Core::Webhook.new(url: "https://example.com/openhub/webhook", value: "sandbox-smoke-test")

qr = client.generate_qr(
  amount: BigDecimal("1.00"), description: "Prueba openhub-bo (ruby)", reference: Time.now.to_i.to_s,
  establishment_id: 1, establishment_name: "Prueba", expires_in: 120, webhook: webhook
)
puts "generated  ref=#{qr.reference} kind=#{qr.kind} status=#{qr.status} expires_at=#{qr.expires_at}"
puts "           png=#{qr.qr_png.bytesize} bytes, amount=#{qr.amount.to_s('F')} #{qr.currency}"

status = client.get_qr_status(qr.reference, kind: qr.kind)
puts "status     #{status.status} (#{status.message})"
raise "expected pending" unless status.status == :pending

cancelled = client.cancel_qr(qr.reference)
puts "cancelled  #{cancelled.status} (#{cancelled.message})"
raise "expected cancelled" unless cancelled.status == :cancelled

begin
  client.cancel_qr(qr.reference)
  abort "re-cancel unexpectedly succeeded"
rescue Core::ApiError => e
  puts "re-cancel  rejected as expected: HTTP #{e.status} #{e.code}"
end
puts "OK"
