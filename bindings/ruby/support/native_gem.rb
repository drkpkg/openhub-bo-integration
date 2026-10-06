# frozen_string_literal: true

# Shared Rake tasks for the openhub-bo native gems.
#
# Each gem's extension depends on Rust crates from <repo>/crates. A published
# gem cannot reach them by relative path, so `rake vendor` copies the needed
# crates into ext/<name>/vendor (resolving `*.workspace = true` fields) and the
# extension's Cargo.toml points there, both in development and in the gem.
require "fileutils"
require "rake/testtask"
require "rbconfig"

module NativeGemTasks
  extend Rake::DSL

  REPO_ROOT = File.expand_path("../../..", __dir__)
  CORE_LIB = File.expand_path("../openhub-bo-core/lib", __dir__)

  module_function

  def define(gem_dir:, pkg:, crates:, gemspec:) # rubocop:disable Metrics/MethodLength
    ext_name = "openhub_bo_#{pkg}"
    ext_dir = File.join(gem_dir, "ext", ext_name)
    vendor_dir = File.join(ext_dir, "vendor")

    desc "Copy the Rust crates the extension needs into #{vendor_dir}"
    task :vendor do
      FileUtils.rm_rf(vendor_dir)
      crates.each { |crate| vendor_crate(crate, File.join(vendor_dir, crate)) }
    end

    # Builds the extension exactly as `gem install` does (extconf.rb + make)
    # and drops it next to the Ruby code for development and tests.
    desc "Compile the native extension into lib/openhub_bo/#{pkg}"
    task compile: :vendor do
      build_dir = File.join(gem_dir, "tmp", ext_name)
      FileUtils.mkdir_p(build_dir)
      Dir.chdir(build_dir) do
        sh RbConfig.ruby, File.join(ext_dir, "extconf.rb")
        sh "make", "-s"
      end
      dlext = RbConfig::CONFIG["DLEXT"]
      FileUtils.cp(File.join(build_dir, "native.#{dlext}"), File.join(gem_dir, "lib", "openhub_bo", pkg, "native.#{dlext}"))
    end

    desc "Remove build artifacts"
    task :clean do
      FileUtils.rm_rf([File.join(gem_dir, "tmp"), vendor_dir, File.join(gem_dir, "pkg")])
      FileUtils.rm_f(Dir[File.join(gem_dir, "lib", "openhub_bo", pkg, "native.*")])
    end

    desc "Build the .gem (vendors the crates first)"
    task build: :vendor do
      FileUtils.mkdir_p(File.join(gem_dir, "pkg"))
      Dir.chdir(gem_dir) { sh "gem", "build", gemspec.loaded_from, "--output", "pkg/#{gemspec.full_name}.gem" }
    end

    Rake::TestTask.new(:test) do |t|
      t.libs = ["lib", "test", CORE_LIB].uniq
      t.test_files = FileList["test/**/test_*.rb"]
      t.warning = false
    end

    task default: %i[compile test]
  end

  def vendor_crate(crate, dest)
    src = File.join(REPO_ROOT, "crates", crate)
    FileUtils.mkdir_p(dest)
    FileUtils.cp_r(File.join(src, "src"), dest)
    manifest = File.read(File.join(src, "Cargo.toml"))
      .gsub(/^(version|edition|rust-version)\.workspace = true$/) { "#{Regexp.last_match(1)} = \"#{workspace_field(Regexp.last_match(1))}\"" }
      .sub(/\n\[dev-dependencies\].*\z/m, "\n")
    File.write(File.join(dest, "Cargo.toml"), manifest)
  end

  def workspace_field(name)
    @workspace ||= File.read(File.join(REPO_ROOT, "Cargo.toml"))
    @workspace[/^#{Regexp.escape(name)} = "([^"]+)"/, 1] or raise "#{name} missing in workspace Cargo.toml"
  end
end
