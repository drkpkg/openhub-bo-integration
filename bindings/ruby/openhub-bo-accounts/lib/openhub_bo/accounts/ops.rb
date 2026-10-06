# frozen_string_literal: true

module OpenhubBo
  module Accounts
    # Operations declared against the native extension (checked at require time).
    module Ops
      NATIVE = Core::NativeBridge.new(Accounts::Native)
      list_of = ->(model) { ->(rows) { rows.map { model.from_h(_1) } } }

      GET = Core::Op.new(NATIVE, "accounts.get") { MerchantAccount.from_h(_1) }
      LIST = Core::Op.new(NATIVE, "accounts.list") { MerchantAccounts.from_h(_1) }
      CREATE = Core::Op.new(NATIVE, "accounts.create") { CreatedAccounts.from_h(_1) }
      SET_STATUS = Core::Op.new(NATIVE, "accounts.set_status", &list_of.(StatusChanged))
      RECONCILE = Core::Op.new(NATIVE, "accounts.reconcile") { Reconciliation.from_h(_1) }
      CREDITS = Core::Op.new(NATIVE, "accounts.credits", &list_of.(Movement))
      DEBITS = Core::Op.new(NATIVE, "accounts.debits", &list_of.(Movement))
      BALANCES = Core::Op.new(NATIVE, "accounts.balances", &list_of.(Balance))

      module_function

      def range(nit, date_from, date_to)
        { "nit" => nit.to_s, "date_from" => Core.iso_date(date_from), "date_to" => Core.iso_date(date_to) }
      end
    end
  end
end
