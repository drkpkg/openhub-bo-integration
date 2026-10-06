# frozen_string_literal: true

module OpenhubBo
  module Accounts
    # Account statuses (symbols). :closed is final and requires zero balance.
    ACCOUNT_STATUSES = %i[active blocked suspended closed].freeze

    # Input of AccountsClient#create_accounts. `alias` and `category` (rubro):
    # max 45 characters each.
    NewAccount = Data.define(:alias, :category) do
      def to_wire = { "alias" => self.alias, "category" => category }
    end

    # Input of AccountsClient#set_account_status. `reason` max 50 characters.
    StatusChange = Data.define(:account_number, :status, :reason) do
      def to_wire = { "account_number" => account_number.to_s, "status" => status.to_s, "reason" => reason }
    end

    Account = Core.model(:number, :alias, :status, :raw_status, casts: { status: :symbol })
    MerchantAccount = Core.model(
      :nit, :merchant_name, :establishment_id, :establishment_name, :account, casts: { account: Account }
    )
    Establishment = Core.model(:id, :name, :accounts, casts: { accounts: [Account] })
    MerchantAccounts = Core.model(:nit, :merchant_name, :establishments, casts: { establishments: [Establishment] }) do
      def accounts = establishments.flat_map(&:accounts)
    end
    CreatedAccounts = Core.model(
      :nit, :merchant_name, :establishment_id, :establishment_name, :accounts, casts: { accounts: [Account] }
    )
    StatusChanged = Core.model(:account_number, :status, :raw_status, casts: { status: :symbol })
    Party = Core.model(:account, :document_id, :name, :bank_code, :bank_name)
    Movement = Core.model(
      :transaction_id, :operation_type, :status, :raw_status, :message, :transaction_at, :amount, :fee, :total,
      :currency, :origin, :destination, :reference, :ach_order_number, :recipient_order_number,
      casts: { status: :symbol, amount: :decimal, fee: :decimal, total: :decimal, origin: Party, destination: Party }
    )
    Balance = Core.model(
      :account_number, :status, :raw_status, :currency, :available, :booked, :held, :last_credit_at, :last_debit_at,
      casts: { status: :symbol, available: :decimal, booked: :decimal, held: :decimal }
    )
    Reconciliation = Core.model(:movements, :balances, casts: { movements: [Movement], balances: [Balance] })
  end
end
