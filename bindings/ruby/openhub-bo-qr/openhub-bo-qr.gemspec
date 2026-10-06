# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name = "openhub-bo-qr"
  spec.version = "0.1.0"
  spec.summary = "Cobros con QR Simple y QR MLD-BCB vía Red Enlace (ATC) OpenHub, Bolivia"
  spec.description = spec.summary
  spec.authors = ["openhub-bo contributors"]
  spec.homepage = "https://github.com/openhub-bo/openhub-bo"
  spec.required_ruby_version = ">= 3.2"
  spec.metadata = {
    "rubygems_mfa_required" => "true",
    "source_code_uri" => spec.homepage
  }
  # TODO: set spec.license once the project chooses one.

  spec.files = Dir[
    "lib/**/*.rb",
    "ext/**/{Cargo.toml,extconf.rb}",
    "ext/**/src/**/*.rs",
    "README.md"
  ]
  spec.require_paths = ["lib"]
  spec.extensions = ["ext/openhub_bo_qr/extconf.rb"]

  spec.add_dependency "openhub-bo-core", "~> 0.1.0"
  spec.add_dependency "bigdecimal", ">= 3.1"
  spec.add_dependency "json", ">= 2.6"
  spec.add_dependency "rb_sys", "~> 0.9"
end
