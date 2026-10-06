# frozen_string_literal: true

require_relative "test_helper"
require "date"

class TestAccounts < Minitest::Test
  Core = OpenhubBo::Core
  Accounts = OpenhubBo::Accounts
  NIT = "1000000019"
  BASE = %r{/cuentas-comercios/v1/cuentas}

  def setup
    @gateway = Core::Testing::MockGateway.new
    @client = Accounts::AccountsClient.new(@gateway.session)
  end

  def stub(method, path, file, status: 200)
    @gateway.stub(method, /#{BASE.source}#{path}\z/, status: status, body: fixture("accounts/#{file}"))
  end

  def body = JSON.parse(@gateway.requests.last["body"])

  def test_lookup_create_and_status
    stub("GET", "/#{NIT}/7011113693", "account_detail_response.json")
    stub("GET", "/#{NIT}", "accounts_list_response.json")
    stub("POST", "", "accounts_create_response.json")
    stub("PATCH", "/estados", "accounts_status_response.json")

    detail = @client.get_account(NIT, "7011113693")
    assert_equal [111_369, :blocked], [detail.establishment_id, detail.account.status]
    assert_equal %w[7011113691 7011113693], @client.list_accounts(NIT).accounts.map(&:number)

    created = @client.create_accounts(NIT, establishment_id: 111_369,
                                           accounts: [Accounts::NewAccount.new(alias: "CAJA 5", category: "COMERCIALES")])
    assert_equal({ "alias" => "CAJA 5", "rubro" => "COMERCIALES", "moneda" => "068" }, body["establecimiento"]["cuenta"][0])
    assert_equal [:active, "CAJA 5 - COMERCIALES"], [created.accounts[0].status, created.accounts[0].alias]

    changed = @client.set_account_status(NIT, [Accounts::StatusChange.new(account_number: "7011113693", status: :blocked,
                                                                          reason: "Sospecha de fraude")])
    assert_equal "PATCH", @gateway.requests.last["method"]
    assert_equal "BLOQUEADA", body["cuentas"][0]["estado"]
    assert_equal :blocked, changed[0].status
  end

  def test_movements_and_balances
    stub("POST", "/transacciones", "reconciliation_response.json")
    stub("POST", "/creditos", "credits_response.json")
    stub("POST", "/debitos", "debits_response.json")
    stub("POST", "/saldos", "balances_response.json")

    rec = @client.reconcile(NIT, date_from: Date.new(2026, 10, 1), date_to: "2026-10-06")
    assert_equal({ "nit" => NIT, "fechaInicio" => "2026-10-01", "fechaFin" => "2026-10-06" }, body)
    assert_equal [2, :paid, BigDecimal("56.86")], [rec.movements.size, rec.movements[0].status, rec.balances[0].available]
    assert_equal "BANCO GANADERO", rec.movements[0].destination.bank_name

    credits = @client.credits(NIT, "7011234561", date_from: "2026-09-10", date_to: "2026-10-06")
    assert_equal ["PAYIN QR", nil, BigDecimal("868.09")], [credits[0].operation_type, credits[0].reference, credits[0].amount]
    assert_equal BigDecimal("299.70"), @client.debits(NIT, "7011234561", date_from: "2026-09-10", date_to: "2026-10-06")[0].total
    balances = @client.balances(NIT, %w[7014227171 7014227172])
    assert_equal ["BOB", nil], [balances[1].currency, balances[1].last_debit_at]
  end

  def test_sandbox_limits_and_errors
    assert_equal "date_to", assert_raises(Core::ValidationError) { @client.reconcile(NIT, date_from: "2026-09-28", date_to: "2026-10-06") }.field
    assert_equal "account_numbers", assert_raises(Core::ValidationError) { @client.balances(NIT, (0..10).map { "70142271#{_1.to_s.rjust(2, '0')}" }) }.field
    assert_equal "nit", assert_raises(Core::ValidationError) { @client.list_accounts("12-3") }.field

    stub("GET", "/1234567", "sandbox_merchant_not_found.json")
    stub("POST", "/saldos", "sandbox_balances_validation.json")
    assert_equal "17", assert_raises(Core::ApiError) { @client.list_accounts("1234567") }.code
    error = assert_raises(Core::ApiError) { @client.balances(NIT, ["7014227171"]) }
    assert_equal ["02", 2], [error.code, error.errors.size]
  end
end
