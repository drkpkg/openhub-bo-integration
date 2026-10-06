# frozen_string_literal: true

module OpenhubBo
  module Accounts
    # Merchant accounts at ATC. Dates accept Date or "yyyy-mm-dd". Sandbox
    # limits: reconciliation at most 7 days, credits/debits 31 days, balances up
    # to 10 accounts.
    class AccountsClient
      attr_reader :session

      def initialize(session)
        @session = session
      end

      def get_account(nit, account_number)
        session.execute(Ops::GET, { "nit" => nit.to_s, "account_number" => account_number.to_s })
      end

      def list_accounts(nit) = session.execute(Ops::LIST, { "nit" => nit.to_s })

      # Creates 1..1000 BOB accounts. Not idempotent: a lost response raises
      # Core::AmbiguousOutcomeError; check list_accounts before retrying.
      def create_accounts(nit, establishment_id:, accounts:)
        session.execute(Ops::CREATE, {
          "nit" => nit.to_s, "establishment_id" => establishment_id, "accounts" => accounts.map(&:to_wire)
        })
      end

      # Changing to :closed requires zero balance and cannot be undone.
      def set_account_status(nit, changes)
        session.execute(Ops::SET_STATUS, { "nit" => nit.to_s, "changes" => changes.map(&:to_wire) })
      end

      # Movements and balances of every account of the merchant (max 7 days).
      def reconcile(nit, date_from:, date_to:) = session.execute(Ops::RECONCILE, Ops.range(nit, date_from, date_to))

      # Incoming movements (QR payins, deposits) of one account (max 31 days).
      def credits(nit, account_number, date_from:, date_to:)
        session.execute(Ops::CREDITS, Ops.range(nit, date_from, date_to).merge("account_number" => account_number.to_s))
      end

      # Outgoing movements (payouts) of one account (max 31 days).
      def debits(nit, account_number, date_from:, date_to:)
        session.execute(Ops::DEBITS, Ops.range(nit, date_from, date_to).merge("account_number" => account_number.to_s))
      end

      # Current balances of up to 10 accounts.
      def balances(nit, account_numbers)
        session.execute(Ops::BALANCES, { "nit" => nit.to_s, "account_numbers" => account_numbers.map(&:to_s) })
      end
    end
  end
end
