#![no_std]

mod errors;
mod event;
mod interface;
mod operations;
mod percentage;
mod storage;
mod testutils;
mod types;

use crate::errors::LPError;
use crate::interface::LiquidityPoolTrait;
use crate::operations::{subtract, sum};
use crate::percentage::{calculate_fees, calculate_repayment_amount, process_lender_contribution};
use crate::storage::{
    check_admin, has_admin, has_borrower, has_lender, read_admin, read_borrower,
    read_contract_balance, read_contributions, read_lender, read_loans, read_token, read_vault,
    remove_borrower, remove_lender, remove_lender_contribution, write_admin, write_borrower,
    write_contract_balance, write_lender, write_lender_contribution, write_loans, write_token,
    write_vault,
};
use crate::types::{Borrower, Lender, LenderStatus, Loan};
use soroban_sdk::{
    contract, contractimpl, contractmeta,
    token::{self},
    Address, Env, Map,
};
use storage::read_loan;

fn token_transfer(env: &Env, from: &Address, to: &Address, amount: &i128) -> Result<(), LPError> {
    let token_id = read_token(env)?;
    let token = token::Client::new(env, &token_id);
    token.transfer(from, to, amount);
    Ok(())
}

fn check_nonnegative_amount(amount: i128) -> Result<(), LPError> {
    if amount < 0 {
        return Err(LPError::AmountMustBePositive);
    }

    Ok(())
}

fn generate_id(env: &Env, loans: &Map<u64, Loan>) -> u64 {
    loop {
        let id: u64 = env.prng().gen();

        if !loans.contains_key(id) {
            return id;
        }
    }
}

contractmeta!(
    key = "Description",
    val = "Liquidity pool for loans with a daily fee of 0.1%"
);

#[contract]
pub struct LiquidityPoolContract;

#[contractimpl]
impl LiquidityPoolTrait for LiquidityPoolContract {
    fn initialize(env: Env, admin: Address, token: Address, vault: Address) -> Result<(), LPError> {
        if has_admin(&env) {
            return Err(LPError::AlreadyInitialized);
        }

        write_admin(&env, &admin);
        write_token(&env, &token);
        write_vault(&env, &vault);
        write_contract_balance(&env, &0i128);

        event::initialize(&env, admin, token, vault);
        Ok(())
    }

    fn balance(env: Env, address: Address) -> Result<i128, LPError> {
        if address == read_admin(&env)? {
            return Ok(read_contract_balance(&env));
        };

        if has_lender(&env, &address) {
            let lender = read_lender(&env, &address)?;
            return Ok(lender.balance);
        }

        Err(LPError::AddressNotRegistered)
    }

    fn deposit(env: Env, address: Address, amount: i128) -> Result<(), LPError> {
        address.require_auth();

        check_nonnegative_amount(amount)?;

        if !has_lender(&env, &address) {
            return Err(LPError::LenderNotRegistered);
        }

        let mut lender = read_lender(&env, &address)?;
        if lender.status != LenderStatus::Enabled {
            return Err(LPError::LenderDisabled);
        };

        let mut total_balance = read_contract_balance(&env);

        total_balance = sum(&total_balance, &amount)?;
        lender.balance = sum(&lender.balance, &amount)?;

        token_transfer(&env, &address, &env.current_contract_address(), &amount)?;

        write_contract_balance(&env, &total_balance);
        write_lender(&env, &address, &lender);

        let mut contributions = read_contributions(&env);

        if !contributions.contains(address.clone()) {
            contributions.push_back(address.clone());
            write_lender_contribution(&env, contributions);
        }

        event::deposit(&env, address, amount);
        Ok(())
    }

    fn withdraw(env: Env, address: Address, amount: i128) -> Result<(), LPError> {
        address.require_auth();

        if !has_lender(&env, &address) {
            return Err(LPError::LenderNotRegistered);
        }

        check_nonnegative_amount(amount)?;

        let mut total_balance = read_contract_balance(&env);
        let mut lender = read_lender(&env, &address)?;

        if amount > lender.balance {
            return Err(LPError::InsufficientBalance);
        }

        if amount > total_balance {
            return Err(LPError::BalanceNotAvailableForAmountRequested);
        }

        total_balance = subtract(&total_balance, &amount)?;
        lender.balance = subtract(&lender.balance, &amount)?;

        token_transfer(&env, &env.current_contract_address(), &address, &amount)?;

        write_contract_balance(&env, &total_balance);
        write_lender(&env, &address, &lender);

        if lender.balance <= 0 {
            remove_lender_contribution(&env, &address)?;
        }

        event::withdraw(&env, address, amount);
        Ok(())
    }

    fn loan(env: Env, address: Address, amount: i128) -> Result<u64, LPError> {
        address.require_auth();

        check_nonnegative_amount(amount)?;
        if !has_borrower(&env, &address) {
            return Err(LPError::BorrowerNotRegistered);
        }

        let borrower = read_borrower(&env, &address)?;

        if !borrower.active {
            return Err(LPError::BorrowerDisabled);
        }

        if amount < borrower.min_withdraw || amount > borrower.max_withdraw {
            return Err(LPError::LoanAmountOutsideWithdrawalLimits);
        }

        let mut total_balance = read_contract_balance(&env);

        if amount > total_balance {
            return Err(LPError::BalanceNotAvailableForAmountRequested);
        }

        let lenders = read_contributions(&env);
        let mut loans = read_loans(&env, &address);

        let lender_contributions =
            process_lender_contribution(&env, lenders.clone(), &amount, &total_balance)?;

        total_balance = subtract(&total_balance, &amount)?;

        let loan_id: u64 = generate_id(&env, &loans);

        let new_loan = Loan {
            amount,
            start_time: env.ledger().timestamp(),
            contributions: lender_contributions,
        };

        loans.set(loan_id, new_loan);

        token_transfer(&env, &env.current_contract_address(), &address, &amount)?;

        write_contract_balance(&env, &total_balance);
        write_loans(&env, &address, &loans);

        event::loan(&env, address, loan_id, amount);
        Ok(loan_id)
    }

    fn repay_loan(env: Env, borrower: Address, loan_id: u64, amount: i128) -> Result<(), LPError> {
        borrower.require_auth();

        check_nonnegative_amount(amount)?;

        if !has_borrower(&env, &borrower) {
            return Err(LPError::BorrowerNotRegistered);
        }

        let mut loans = read_loans(&env, &borrower);
        let mut loan = read_loan(&loans, &loan_id)?;

        let vault = read_vault(&env)?;
        let total_fees = calculate_fees(&env, &loan)?;
        let admin_fees = total_fees / 10;
        let mut amount_for_lenders = subtract(&amount, &admin_fees)?;

        let mut total_balance = read_contract_balance(&env);

        for (address, percentage) in loan.contributions.iter() {
            let mut lender = read_lender(&env, &address)?;
            lender.active_loans = subtract(&lender.active_loans, &1)?;
            lender.balance = sum(
                &lender.balance,
                &(calculate_repayment_amount(amount_for_lenders, percentage)?),
            )?;

            if lender.active_loans == 0 && lender.status == LenderStatus::PendingRemoval {
                token_transfer(
                    &env,
                    &env.current_contract_address(),
                    &address,
                    &lender.balance,
                )?;

                amount_for_lenders = subtract(&amount_for_lenders, &lender.balance)?;
                remove_lender(&env, &address);
            } else {
                write_lender(&env, &address, &lender);
            }
        }

        let repay_loan_amount = sum(&loan.amount, &(calculate_fees(&env, &loan)?))?;
        let repay_amount_diff = subtract(&repay_loan_amount, &amount)?;
        total_balance = sum(&total_balance, &amount_for_lenders)?;

        token_transfer(&env, &borrower, &env.current_contract_address(), &amount)?;
        token_transfer(&env, &env.current_contract_address(), &vault, &admin_fees)?;

        if repay_amount_diff > 0 {
            loan.amount = repay_amount_diff;
            loans.set(loan_id, loan);
        } else {
            loans.remove(loan_id);
        }

        write_contract_balance(&env, &total_balance);
        write_loans(&env, &borrower, &loans);
        event::repay_loan(&env, borrower, loan_id, amount);
        Ok(())
    }

    fn repay_loan_amount(env: Env, borrower: Address, loan_id: u64) -> Result<i128, LPError> {
        borrower.require_auth();

        if !has_borrower(&env, &borrower) {
            return Err(LPError::BorrowerNotRegistered);
        }

        let loans = read_loans(&env, &borrower);
        let loan = read_loan(&loans, &loan_id)?;

        let loan_amount = sum(&loan.amount, &calculate_fees(&env, &loan)?)?;

        Ok(loan_amount)
    }

    fn get_loan_withdraw_limit(env: Env, address: Address) -> Result<(i128, i128), LPError> {
        address.require_auth();

        if !has_borrower(&env, &address) {
            return Err(LPError::BorrowerNotRegistered);
        }

        let borrower = read_borrower(&env, &address)?;

        Ok((borrower.min_withdraw, borrower.max_withdraw))
    }

    fn add_borrower(
        env: Env,
        address: Address,
        min_amount: i128,
        max_amount: i128,
    ) -> Result<(), LPError> {
        let admin = check_admin(&env)?;

        check_nonnegative_amount(min_amount)?;
        check_nonnegative_amount(max_amount)?;

        if has_borrower(&env, &address) {
            return Err(LPError::BorrowerAlreadyRegistered);
        }

        let (min_withdraw, max_withdraw) = if min_amount <= max_amount {
            (min_amount, max_amount)
        } else {
            (max_amount, min_amount)
        };

        let borrower = Borrower {
            active: true,
            min_withdraw,
            max_withdraw,
        };

        write_borrower(&env, &address, borrower);

        event::add_borrower(&env, admin, address, (min_withdraw, max_withdraw));
        Ok(())
    }

    fn set_borrower_status(env: Env, address: Address, active: bool) -> Result<(), LPError> {
        let admin = check_admin(&env)?;

        if !has_borrower(&env, &address) {
            return Err(LPError::BorrowerNotRegistered);
        }

        let mut borrower = read_borrower(&env, &address)?;
        borrower.active = active;

        write_borrower(&env, &address, borrower);

        event::set_borrower_status(&env, admin, address, active);
        Ok(())
    }

    fn set_borrower_limits(
        env: Env,
        address: Address,
        min_amount: i128,
        max_amount: i128,
    ) -> Result<(), LPError> {
        let admin = check_admin(&env)?;

        check_nonnegative_amount(min_amount)?;
        check_nonnegative_amount(max_amount)?;

        if !has_borrower(&env, &address) {
            return Err(LPError::BorrowerNotRegistered);
        }

        let mut borrower = read_borrower(&env, &address)?;

        let (min_withdraw, max_withdraw) = if min_amount <= max_amount {
            (min_amount, max_amount)
        } else {
            (max_amount, min_amount)
        };

        borrower.min_withdraw = min_withdraw;
        borrower.max_withdraw = max_withdraw;

        write_borrower(&env, &address, borrower);

        event::set_borrower_limits(&env, admin, address, (min_withdraw, max_withdraw));
        Ok(())
    }

    fn remove_borrower(env: Env, borrower: Address) -> Result<(), LPError> {
        let admin = check_admin(&env)?;

        if !has_borrower(&env, &borrower) {
            return Err(LPError::BorrowerNotRegistered);
        }

        remove_borrower(&env, &borrower);

        event::remove_borrower(&env, admin, borrower);
        Ok(())
    }

    fn add_lender(env: Env, lender: Address) -> Result<(), LPError> {
        let admin = check_admin(&env)?;

        if has_lender(&env, &lender) {
            return Err(LPError::LenderAlreadyRegistered);
        }

        let data = Lender {
            status: LenderStatus::Enabled,
            balance: 0,
            active_loans: 0,
        };

        write_lender(&env, &lender, &data);

        event::add_lender(&env, admin, lender);
        Ok(())
    }

    fn set_lender_status(env: Env, address: Address, active: bool) -> Result<(), LPError> {
        let admin = check_admin(&env)?;

        if !has_lender(&env, &address) {
            return Err(LPError::LenderNotRegistered);
        }

        let mut lender = read_lender(&env, &address)?;
        lender.status = if active {
            LenderStatus::Enabled
        } else {
            LenderStatus::Disabled
        };

        let mut contributions = read_contributions(&env);

        if lender.status == LenderStatus::Enabled {
            if !contributions.contains(&address) {
                contributions.push_back(address.clone());
            }
        } else if let Some(index) = contributions
            .iter()
            .position(|addr| addr == address.clone())
        {
            contributions.remove(index as u32);
        }

        write_lender(&env, &address, &lender);
        write_lender_contribution(&env, contributions);

        event::set_lender_status(&env, admin, address, lender.status);
        Ok(())
    }

    fn remove_lender(env: Env, address: Address) -> Result<(), LPError> {
        let admin = check_admin(&env)?;

        if !has_lender(&env, &address) {
            return Err(LPError::LenderNotRegistered);
        }

        let mut total_balance = read_contract_balance(&env);
        let mut lender = read_lender(&env, &address)?;

        if lender.balance > total_balance {
            return Err(LPError::BalanceNotAvailableForAmountRequested);
        }

        total_balance = subtract(&total_balance, &lender.balance)?;
        lender.balance = subtract(&lender.balance, &lender.balance)?;

        token_transfer(
            &env,
            &env.current_contract_address(),
            &address,
            &lender.balance,
        )?;

        if lender.active_loans > 0 {
            lender.status = LenderStatus::PendingRemoval;
            write_lender(&env, &address, &lender);
        } else {
            remove_lender(&env, &address);
        }

        write_contract_balance(&env, &total_balance);
        remove_lender_contribution(&env, &address)?;

        event::remove_lender(&env, admin, address);
        Ok(())
    }
}

mod test;
