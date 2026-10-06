# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name = "openhub-bo-fx"
  spec.version = "0.1.0"
  spec.summary = "Cobros QR con PIX, activos virtuales (USDT/USDC) y Binance Pay vía Red Enlace (ATC) OpenHub, Bolivia"
  spec.description = "#{spec.summary}. Núcleo en Rust compartido con los paquetes openhub-bo de Python y TypeScript. " \
                     "Integración no oficial, sin afiliación con Red Enlace (ATC)."
  spec.authors = ["Felix Daniel Coca Calvimontes"]
  spec.homepage = "https://github.com/drkpkg/openhub-bo-integration"
  spec.required_ruby_version = ">= 3.2"
  spec.metadata = {
    "rubygems_mfa_required" => "true",
    "source_code_uri" => spec.homepage,
    "bug_tracker_uri" => "#{spec.homepage}/issues"
  }
  spec.license = "Apache-2.0"

  spec.files = Dir[
    "lib/**/*.rb",
    "Cargo.{toml,lock}",
    "ext/**/{Cargo.toml,extconf.rb}",
    "ext/**/src/**/*.rs",
    "README.md",
    "LICENSE",
    "NOTICE"
  ]
  spec.require_paths = ["lib"]
  spec.extensions = ["ext/openhub_bo_fx/extconf.rb"]

  spec.add_dependency "openhub-bo-core", "~> 0.1.0"
  spec.add_dependency "bigdecimal", ">= 3.1", "< 5"
  spec.add_dependency "json", ">= 2.6", "< 4"
  spec.add_dependency "rb_sys", "~> 0.9"
end
