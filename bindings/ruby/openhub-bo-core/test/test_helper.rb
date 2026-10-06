# frozen_string_literal: true

require "minitest/autorun"
require "openhub_bo/core"
require "openhub_bo/core/testing"

FIXTURES = File.expand_path("../../../../fixtures/openhub", __dir__)

def fixture(path) = File.read(File.join(FIXTURES, path))

# A stand-in native module with one non-idempotent operation (`fake.write`)
# and an ATC-style timeout, to exercise Session paths core alone cannot.
module FakeNative
  module_function

  def protocol_version = OpenhubBo::Core::SUPPORTED_PROTOCOL
  def version = "0.0.0-test"

  def call(op, payload)
    data = JSON.parse(payload)
    value =
      case op
      when "describe"
        [{ "kind" => "operation", "name" => "fake.write", "idempotent" => false, "timeout_secs" => 90 }]
      when "fake.write.build"
        {
          "method" => "POST",
          "url" => "#{data['config']['base_url']}/fake",
          "headers" => { "access_token" => data["token"]["access_token"], "client_id" => data["config"]["client_id"] },
          "body" => "{}"
        }
      when "fake.write.parse"
        status = data["response"]["status"]
        return error("authentication", "status" => status, "message" => "bad token") if status == 401
        return error("api", "status" => status, "message" => "down", "errors" => [], "retryable" => true) if status >= 500

        { "done" => true }
      end
    JSON.generate({ "ok" => true, "value" => value })
  end

  def error(kind, fields) = JSON.generate({ "ok" => false, "error" => { "kind" => kind }.merge(fields) })
end
