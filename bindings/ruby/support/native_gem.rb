# frozen_string_literal: true

# Shared Rake tasks for the openhub-bo native gems (rb_sys + rake-compiler).
#
# Each gem's extension depends on Rust crates from <repo>/crates. A published
# gem cannot reach them by relative path, so they are vendored into
# ext/<name>/vendor (resolving `*.workspace = true` fields) every time the
# Rakefile loads inside the repo; the extension's Cargo.toml points there, both
# in development and in the gem. Inside the rb-sys-dock cross-compilation
# container only the gem directory is mounted, so the vendored copy made on the
# host is used as is.
#
#   rake compile                 build the extension for this Ruby (lib/openhub_bo/<pkg>/)
#   rake test                    run the tests against it
#   rake gem                     source gem (compiled on install, needs Rust)
#   rake native:<platform> gem   precompiled gem (run by rb-sys-dock, see release.yml)
require "fileutils"
require "rake/clean"
require "rake/testtask"
require "rubygems/package_task"
require "rb_sys/extensiontask"

module NativeGemTasks
  extend Rake::DSL

  REPO_ROOT = File.expand_path("../../..", __dir__)
  CORE_LIB = File.expand_path("../openhub-bo-core/lib", __dir__)

  # Platforms with precompiled gems; any other platform installs the source gem.
  PLATFORMS = %w[
    x86_64-linux
    x86_64-linux-musl
    aarch64-linux
    aarch64-linux-musl
    x86_64-darwin
    arm64-darwin
    x64-mingw-ucrt
  ].freeze

  module_function

  def define(gem_dir:, pkg:, crates:, gemspec_path:)
    ext_name = "openhub_bo_#{pkg}"
    vendor_dir = File.join(gem_dir, "ext", ext_name, "vendor")
    vendor(crates, vendor_dir) if File.directory?(File.join(REPO_ROOT, "crates"))
    abort "#{vendor_dir} missing: run rake inside the repository first" unless File.directory?(vendor_dir)

    # Loaded after vendoring so the gem's file list includes the vendored crates.
    gemspec = Gem::Specification.load(gemspec_path)
    Gem::PackageTask.new(gemspec).define

    RbSys::ExtensionTask.new(ext_name, gemspec) do |ext|
      ext.lib_dir = "lib/openhub_bo/#{pkg}"
      ext.cross_compile = true
      ext.cross_platform = PLATFORMS
    end

    Rake::TestTask.new(:test) do |t|
      t.libs = ["lib", "test", CORE_LIB].uniq
      t.test_files = FileList["test/**/test_*.rb"]
      t.warning = false
    end

    desc "Vendor the repo crates into ext/#{ext_name}/vendor (also done whenever this Rakefile loads)"
    task :vendor

    desc "Alias of `gem`: build the source gem into pkg/"
    task build: :gem

    task default: %i[compile test]
  end

  def vendor(crates, vendor_dir)
    FileUtils.rm_rf(vendor_dir)
    crates.each { |crate| vendor_crate(crate, File.join(vendor_dir, crate)) }
  end

  def vendor_crate(crate, dest)
    src = File.join(REPO_ROOT, "crates", crate)
    FileUtils.mkdir_p(dest)
    FileUtils.cp_r(File.join(src, "src"), dest)
    manifest = File.read(File.join(src, "Cargo.toml"))
                   .gsub(/^(version|edition|rust-version|license|repository)\.workspace = true$/) do
                     "#{Regexp.last_match(1)} = \"#{workspace_field(Regexp.last_match(1))}\""
                   end
                   .sub(/\n\[dev-dependencies\].*\z/m, "\n")
    File.write(File.join(dest, "Cargo.toml"), manifest)
  end

  def workspace_field(name)
    @workspace ||= File.read(File.join(REPO_ROOT, "Cargo.toml"))
    @workspace[/^#{Regexp.escape(name)} = "([^"]+)"/, 1] or raise "#{name} missing in workspace Cargo.toml"
  end
end
