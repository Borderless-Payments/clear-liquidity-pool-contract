use soroban_sdk::{Address, Env, Map, Vec};

use crate::{
    errors::LPError,
    types::{Borrower, DataKey, Lender, Loan},
};

type Loans = Map<u64, Loan>;

pub fn check_admin(env: &Env) -> Result<Address, LPError> {
    let admin = read_admin(env)?;
    admin.require_auth();
    Ok(admin)
}

pub fn has_admin(env: &Env) -> bool {
    env.storage().persistent().has(&DataKey::Admin)
}

pub fn read_admin(env: &Env) -> Result<Address, LPError> {
    match env.storage().persistent().get(&DataKey::Admin) {
        Some(admin) => Ok(admin),
        None => Err(LPError::AdminNotFound),
    }
}

pub fn has_borrower(env: &Env, borrower: &Address) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Borrower(borrower.clone()))
}

pub fn has_lender(env: &Env, lender: &Address) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Lender(lender.clone()))
}

pub fn read_borrower(env: &Env, borrower: &Address) -> Result<Borrower, LPError> {
    env.storage()
        .persistent()
        .get(&DataKey::Borrower(borrower.clone()))
        .ok_or(LPError::BorrowerNotFound)
}

pub fn read_contract_balance(env: &Env) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::TotalBalance)
        .unwrap_or(0)
}

pub fn read_contributions(env: &Env) -> Vec<Address> {
    env.storage()
        .persistent()
        .get(&DataKey::Contribution)
        .unwrap_or(Vec::new(env))
}

pub fn read_loan(loans: &Loans, loan_id: &u64) -> Result<Loan, LPError> {
    let loan = loans
        .try_get(*loan_id)
        .map_err(|_| LPError::LoanNotFoundOrExists)?
        .ok_or(LPError::LoanNotFoundOrExists)?;
    Ok(loan)
}

pub fn read_loans(env: &Env, address: &Address) -> Loans {
    env.storage()
        .persistent()
        .get(&DataKey::Loan(address.clone()))
        .unwrap_or(Map::new(env))
}

pub fn read_lender(env: &Env, lender: &Address) -> Result<Lender, LPError> {
    env.storage()
        .persistent()
        .get(&DataKey::Lender(lender.clone()))
        .ok_or(LPError::LenderNotFound)
}

pub fn read_token(env: &Env) -> Result<Address, LPError> {
    match env.storage().persistent().get(&DataKey::Token) {
        Some(token) => Ok(token),
        None => Err(LPError::TokenNotFound),
    }
}

pub fn read_vault(env: &Env) -> Result<Address, LPError> {
    match env.storage().persistent().get(&DataKey::Vault) {
        Some(vault) => Ok(vault),
        None => Err(LPError::VaultNotFound),
    }
}

pub fn remove_borrower(env: &Env, borrower: &Address) {
    env.storage()
        .persistent()
        .remove(&DataKey::Borrower(borrower.clone()))
}

pub fn remove_lender(env: &Env, lender: &Address) {
    env.storage()
        .persistent()
        .remove(&DataKey::Lender(lender.clone()))
}

pub fn remove_lender_contribution(env: &Env, lender: &Address) -> Result<(), LPError> {
    let mut contributions = read_contributions(env);

    if let Some(index) = contributions.iter().position(|address| &address == lender) {
        match index.try_into() {
            Ok(valid_index) => contributions.remove(valid_index),
            Err(_) => return Err(LPError::LenderNotFoundInContributions),
        };
    }

    write_lender_contribution(env, contributions);
    Ok(())
}

pub fn write_admin(env: &Env, admin: &Address) {
    env.storage().persistent().set(&DataKey::Admin, admin);
}

pub fn write_borrower(env: &Env, address: &Address, borrower: Borrower) {
    env.storage()
        .persistent()
        .set(&DataKey::Borrower(address.clone()), &borrower);
}

pub fn write_contract_balance(env: &Env, amount: &i128) {
    env.storage()
        .persistent()
        .set(&DataKey::TotalBalance, amount);
}

pub fn write_loans(env: &Env, address: &Address, loans: &Map<u64, Loan>) {
    env.storage()
        .persistent()
        .set(&DataKey::Loan(address.clone()), loans);
}

pub fn write_lender(env: &Env, lender: &Address, data: &Lender) {
    env.storage()
        .persistent()
        .set(&DataKey::Lender(lender.clone()), data);
}

pub fn write_lender_contribution(env: &Env, contributions: Vec<Address>) {
    env.storage()
        .persistent()
        .set(&DataKey::Contribution, &contributions);
}

pub fn write_token(env: &Env, address: &Address) {
    env.storage().persistent().set(&DataKey::Token, address);
}

pub fn write_vault(env: &Env, address: &Address) {
    env.storage().persistent().set(&DataKey::Vault, address);
}
