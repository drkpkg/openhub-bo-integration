# frozen_string_literal: true

require "minitest/autorun"
require "openhub_bo/fx"
require "openhub_bo/core/testing"

FIXTURES = File.expand_path("../../../../fixtures/openhub", __dir__)

def fixture(path) = File.read(File.join(FIXTURES, path))
