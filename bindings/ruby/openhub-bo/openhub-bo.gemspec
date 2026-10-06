# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name = "openhub-bo"
  spec.version = "0.1.0"
  spec.summary = "Todos los clientes openhub-bo para Red Enlace (ATC) OpenHub, Bolivia (QR, FX, cuentas, pagos)"
  spec.description = "Metagema: instala openhub-bo-core, -qr, -fx, -accounts y -payouts. " \
                     "Si solo necesitas un producto, instala solo esa gema."
  spec.authors = ["Felix Daniel Coca Calvimontes"]
  spec.homepage = "https://github.com/drkpkg/openhub-bo-integration"
  spec.required_ruby_version = ">= 3.2"
  spec.metadata = {
    "rubygems_mfa_required" => "true",
    "source_code_uri" => spec.homepage,
    "bug_tracker_uri" => "#{spec.homepage}/issues"
  }
  spec.license = "Apache-2.0"

  spec.files = Dir["lib/**/*.rb", "README.md", "LICENSE", "NOTICE"]
  spec.require_paths = ["lib"]

  %w[core qr fx accounts payouts].each { |pkg| spec.add_dependency "openhub-bo-#{pkg}", "~> 0.1.0" }
end
