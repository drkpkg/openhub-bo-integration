# frozen_string_literal: true

require "date"
require "time"

module OpenhubBo
  module Core
    module Environment
      DEVELOPMENT = :development
      SANDBOX = :sandbox
      PRODUCTION = :production
      ALL = [DEVELOPMENT, SANDBOX, PRODUCTION].freeze
    end

    # Status shared by every product family (symbols); `raw_status` keeps the original.
    module PaymentStatus
      ALL = %i[pending processing paid cancelled expired rejected reversed error unknown].freeze
      NOT_FINAL = %i[pending processing unknown].freeze

      def self.final?(status) = !NOT_FINAL.include?(status)
    end

    # OpenHub timestamps carry no offset: they are Bolivia time.
    BOLIVIA_OFFSET = "-04:00"

    # Where ATC must POST notifications. ATC sends `key: value` as a header; it is
    # the only authentication, so use a long random value.
    Webhook = Data.define(:url, :value, :key) do
      def initialize(url:, value:, key: "x-api-key") = super

      def to_wire = { "url" => url, "key" => key, "value" => value }

      def inspect = "#<#{self.class.name} url=#{url.inspect} key=#{key.inspect} value=[FILTERED]>"
      alias_method :to_s, :inspect
    end

    module_function

    # Amounts cross the native boundary as exact decimal strings.
    def amount(value)
      case value
      when BigDecimal then value.to_s("F")
      when Integer then value.to_s
      when String then value
      else
        raise TypeError,
              "amount must be BigDecimal, Integer or String, not #{value.class} " \
              "(floats are rejected to avoid rounding errors)"
      end
    end

    def seconds(value)
      raise TypeError, "expected Integer seconds, not #{value.class}" unless value.is_a?(Integer)

      value
    end

    def iso_date(value) = value.is_a?(Date) ? value.iso8601 : value

    def decimal(value) = value.nil? ? nil : BigDecimal(value)

    # Parses an OpenHub timestamp, assuming Bolivia time when it has no offset.
    def local_time(value)
      return nil if value.nil? || value.empty?

      has_offset = value.match?(/(Z|[+-]\d{2}:?\d{2})\z/)
      Time.iso8601(has_offset ? value : value.tr(" ", "T") + BOLIVIA_OFFSET)
    rescue ArgumentError
      nil
    end

    # Defines an immutable value class from the core's snake_case JSON.
    #
    #   Payer = Core.model(:name, :document_id)
    #   Qr = Core.model(:reference, :amount, :payer, casts: { amount: :decimal, payer: Payer })
    #
    # Casts: :decimal (BigDecimal), :symbol, :time (Bolivia time), a model class,
    # or [ModelClass] for lists.
    def model(*fields, casts: {}, &block)
      klass = Data.define(*fields, &block)
      klass.define_singleton_method(:from_h) do |hash|
        return nil if hash.nil?

        attrs = fields.to_h { |field| [field, Core.cast(hash[field.to_s], casts[field])] }
        new(**attrs)
      end
      klass
    end

    def cast(value, how)
      return value if how.nil? || value.nil?

      case how
      when :decimal then BigDecimal(value.to_s)
      when :symbol then value.to_sym
      when :time then local_time(value)
      when Array then value.map { |item| how.first.from_h(item) }
      else how.from_h(value)
      end
    end
  end
end
