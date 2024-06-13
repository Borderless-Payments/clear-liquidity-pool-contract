#![cfg(test)]

use super::percentage::percentage_to_integer;
use super::testutils::{create_token_contract, Setup};
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn test_initialize() {
    let setup = Setup::new();

    assert_eq!(setup.liquid_contract.read_contract_balance(), 0i128);
    assert_eq!(setup.liquid_contract.read_admin(), setup.admin);
    assert_eq!(setup.liquid_contract.read_token(), setup.token.address);
}

#[test]
#[should_panic(expected = "contract already initialized with an admin")]
fn test_already_initialize() {
    let setup = Setup::new();

    let env = Env::default();
    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client) = create_token_contract(&env, &token_admin);

    setup
        .liquid_contract
        .client()
        .initialize(&admin, &token.address);
}

#[test]
fn test_balance_with_admin() {
    let setup = Setup::new();

    let balance = setup.liquid_contract.client().balance(&setup.admin);

    assert_eq!(setup.liquid_contract.read_contract_balance(), balance);
}

#[test]
fn test_balance_with_lender() {
    let setup = Setup::new();
    let lender = Address::generate(&setup.env);

    setup
        .liquid_contract
        .client()
        .add_lender(&setup.admin, &lender);

    let balance = setup.liquid_contract.client().balance(&lender);

    assert_eq!(setup.liquid_contract.read_lender(&lender), balance);
}

#[test]
#[should_panic(expected = "address is not registered")]
fn test_balance_without_registered_address() {
    let setup = Setup::new();

    let unregistered_address = Address::generate(&setup.env);

    setup
        .liquid_contract
        .client()
        .balance(&unregistered_address);
}

#[test]
fn test_deposit() {
    let setup = Setup::new();
    let lender1 = Address::generate(&setup.env);
    let lender2 = Address::generate(&setup.env);

    setup
        .liquid_contract
        .client()
        .add_lender(&setup.admin, &lender1);

    setup
        .liquid_contract
        .client()
        .add_lender(&setup.admin, &lender2);

    setup.token_admin.mock_all_auths().mint(&lender1, &4i128);
    setup.token_admin.mock_all_auths().mint(&lender2, &7i128);

    setup
        .liquid_contract
        .client()
        .mock_all_auths()
        .deposit(&lender1, &4i128);

    assert_eq!(
        setup.liquid_contract.read_lender_contribution(&lender1),
        percentage_to_integer(100.0)
    );

    setup
        .liquid_contract
        .client()
        .mock_all_auths()
        .deposit(&lender2, &7i128);

    assert_eq!(setup.liquid_contract.read_contract_balance(), 11i128);
    assert_eq!(setup.liquid_contract.read_lender(&lender1), 4i128);
    assert_eq!(
        setup.liquid_contract.read_lender_contribution(&lender1),
        percentage_to_integer(36.3636363)
    );
    assert_eq!(setup.liquid_contract.read_lender(&lender2), 7i128);
    assert_eq!(
        setup.liquid_contract.read_lender_contribution(&lender2),
        percentage_to_integer(63.6363636)
    );
}

#[test]
#[should_panic(expected = "lender is not registered")]
fn test_deposit_without_lender() {
    let setup = Setup::new();
    let lender = Address::generate(&setup.env);

    setup
        .liquid_contract
        .client()
        .mock_all_auths()
        .deposit(&lender, &10i128);
}

#[test]
#[should_panic(expected = "amount must be positive")]
fn test_deposit_with_negative_amount() {
    let setup = Setup::new();
    let lender = Address::generate(&setup.env);

    setup
        .liquid_contract
        .client()
        .add_lender(&setup.admin, &lender);

    setup
        .liquid_contract
        .client()
        .mock_all_auths()
        .deposit(&lender, &-10i128);
}

#[test]
fn test_withdraw() {
    let setup = Setup::new();
    let lender1 = Address::generate(&setup.env);
    let lender2 = Address::generate(&setup.env);

    setup
        .liquid_contract
        .client()
        .add_lender(&setup.admin, &lender1);

    setup
        .liquid_contract
        .client()
        .add_lender(&setup.admin, &lender2);

    setup.token_admin.mock_all_auths().mint(&lender1, &10i128);
    setup.token_admin.mock_all_auths().mint(&lender2, &10i128);
    setup
        .token_admin
        .mock_all_auths()
        .mint(&setup.liquid_contract_id, &20i128);

    setup
        .liquid_contract
        .client()
        .mock_all_auths()
        .deposit(&lender1, &10i128);

    assert_eq!(
        setup.liquid_contract.read_lender_contribution(&lender1),
        percentage_to_integer(100.0)
    );

    setup
        .liquid_contract
        .client()
        .mock_all_auths()
        .deposit(&lender2, &10i128);

    assert_eq!(setup.liquid_contract.read_contract_balance(), 20i128);
    assert_eq!(
        setup.liquid_contract.read_lender_contribution(&lender1),
        percentage_to_integer(50.0)
    );
    assert_eq!(
        setup.liquid_contract.read_lender_contribution(&lender2),
        percentage_to_integer(50.0)
    );

    setup
        .liquid_contract
        .client()
        .mock_all_auths()
        .withdraw(&lender1, &5i128);

    setup
        .liquid_contract
        .client()
        .mock_all_auths()
        .withdraw(&lender2, &7i128);

    assert_eq!(setup.liquid_contract.read_contract_balance(), 8i128);
    assert_eq!(setup.liquid_contract.read_lender(&lender1), 5i128);
    assert_eq!(setup.liquid_contract.read_lender(&lender2), 3i128);
    assert_eq!(
        setup.liquid_contract.read_lender_contribution(&lender1),
        percentage_to_integer(62.4999999)
    );
    assert_eq!(
        setup.liquid_contract.read_lender_contribution(&lender2),
        percentage_to_integer(37.5)
    );
}

#[test]
#[should_panic(expected = "lender is not registered")]
fn test_withdraw_without_lender() {
    let setup = Setup::new();
    let lender = Address::generate(&setup.env);

    setup
        .liquid_contract
        .client()
        .mock_all_auths()
        .withdraw(&lender, &7i128);
}

#[test]
#[should_panic(expected = "amount must be positive")]
fn test_withdraw_negative_amount() {
    let setup = Setup::new();
    let lender = Address::generate(&setup.env);

    setup
        .liquid_contract
        .client()
        .add_lender(&setup.admin, &lender);

    setup.token_admin.mock_all_auths().mint(&lender, &7i128);

    setup
        .liquid_contract
        .client()
        .mock_all_auths()
        .deposit(&lender, &7i128);

    setup
        .liquid_contract
        .client()
        .mock_all_auths()
        .withdraw(&lender, &-7i128);
}

#[test]
#[should_panic(expected = "balance not available for the amount requested")]
fn test_withdraw_amount_greater_lender_balance() {
    let setup = Setup::new();
    let lender = Address::generate(&setup.env);

    setup
        .liquid_contract
        .client()
        .add_lender(&setup.admin, &lender);

    setup.token_admin.mock_all_auths().mint(&lender, &7i128);

    setup
        .liquid_contract
        .client()
        .mock_all_auths()
        .deposit(&lender, &7i128);

    setup
        .liquid_contract
        .client()
        .mock_all_auths()
        .withdraw(&lender, &10i128);
}

#[test]
fn test_add_borrower() {
    let setup = Setup::new();

    let borrower = Address::generate(&setup.env);

    setup
        .liquid_contract
        .client()
        .add_borrower(&setup.admin, &borrower);

    assert!(setup.liquid_contract.has_borrower(&borrower));
}

#[test]
#[should_panic(expected = "only the stored admin can add borrowers")]
fn test_add_borrower_with_fake_admin() {
    let setup = Setup::new();

    let fake_admin = Address::generate(&setup.env);
    let borrower = Address::generate(&setup.env);

    setup
        .liquid_contract
        .client()
        .add_borrower(&fake_admin, &borrower);
}

#[test]
#[should_panic(expected = "borrower is already registered")]
fn test_add_registered_borrower() {
    let setup = Setup::new();

    let borrower = Address::generate(&setup.env);

    setup
        .liquid_contract
        .client()
        .add_borrower(&setup.admin, &borrower);

    setup
        .liquid_contract
        .client()
        .add_borrower(&setup.admin, &borrower);
}

#[test]
fn test_remove_borrower() {
    let setup = Setup::new();

    let borrower = Address::generate(&setup.env);

    setup
        .liquid_contract
        .client()
        .add_borrower(&setup.admin, &borrower);

    assert!(setup.liquid_contract.has_borrower(&borrower));

    setup
        .liquid_contract
        .client()
        .remove_borrower(&setup.admin, &borrower);

    assert!(!setup.liquid_contract.has_borrower(&borrower));
}

#[test]
#[should_panic(expected = "borrower is not registered")]
fn test_remove_without_borrower() {
    let setup = Setup::new();

    let borrower = Address::generate(&setup.env);

    setup
        .liquid_contract
        .client()
        .remove_borrower(&setup.admin, &borrower);
}

#[test]
#[should_panic(expected = "only the stored admin can add borrowers")]
fn test_remove_borrower_with_fake_admin() {
    let setup = Setup::new();

    let fake_admin = Address::generate(&setup.env);
    let borrower = Address::generate(&setup.env);

    setup
        .liquid_contract
        .client()
        .remove_borrower(&fake_admin, &borrower);
}

#[test]
fn test_add_lender() {
    let setup = Setup::new();

    let lender = Address::generate(&setup.env);

    setup
        .liquid_contract
        .client()
        .add_lender(&setup.admin, &lender);

    assert!(setup.liquid_contract.has_lender(&lender));
}

#[test]
#[should_panic(expected = "only the stored admin can add lenders")]
fn test_add_lender_with_fake_admin() {
    let setup = Setup::new();

    let fake_admin = Address::generate(&setup.env);
    let lender = Address::generate(&setup.env);

    setup
        .liquid_contract
        .client()
        .add_lender(&fake_admin, &lender);
}

#[test]
fn test_remove_lender() {
    let setup = Setup::new();

    let lender = Address::generate(&setup.env);

    setup
        .liquid_contract
        .client()
        .add_lender(&setup.admin, &lender);

    assert!(setup.liquid_contract.has_lender(&lender));

    setup
        .liquid_contract
        .client()
        .remove_lender(&setup.admin, &lender);

    assert!(!setup.liquid_contract.has_lender(&lender));
}

#[test]
#[should_panic(expected = "only the stored admin can add lenders")]
fn test_remove_lender_with_fake_admin() {
    let setup = Setup::new();

    let fake_admin = Address::generate(&setup.env);
    let lender = Address::generate(&setup.env);

    setup
        .liquid_contract
        .client()
        .remove_lender(&fake_admin, &lender);
}

#[test]
#[should_panic(expected = "lender is not registered")]
fn test_remove_without_lender() {
    let setup = Setup::new();

    let lender = Address::generate(&setup.env);

    setup
        .liquid_contract
        .client()
        .remove_lender(&setup.admin, &lender);
}
