use soroban_sdk::{Address, Env, Map, Vec};

use crate::errors::LPError;
use crate::operations::{divide, multiply, subtract, sum};
use crate::storage::{read_lender, write_lender};
use crate::types::{LenderStatus, Loan};

type ContributionsMap = Map<Address, i64>;
type ContributionResult = Result<ContributionsMap, LPError>;

pub(crate) const ONE_XLM_IN_STROOPS: i64 = 10_000_000;
const INTEREST_RATE_PER_DAY: u64 = 100;
const SECONDS_PER_DAY: u64 = 86400;
const TOTAL_BASIS_PERCENTAGE: u64 = 100_000;
const MINIMUM_DURATION_DAYS: u64 = 1;

fn to_fixed_point() -> Result<i128, LPError> {
    multiply(&100, &(ONE_XLM_IN_STROOPS as i128))
}

pub fn calculate_fees(env: &Env, loan: &Loan) -> Result<i128, LPError> {
    let now_ledger = env.ledger().timestamp();
    let start_time = loan.start_time;
    let duration_seconds = subtract(&now_ledger, &start_time)?;

    let divisor = subtract(&now_ledger, &start_time)?;
    let duration_days= if duration_seconds < SECONDS_PER_DAY {
        MINIMUM_DURATION_DAYS
    }else{
        divide(&divisor, &SECONDS_PER_DAY)?
    };
    let interest_loan = multiply(&INTEREST_RATE_PER_DAY, &duration_days)?;
    let fees_fixed_point = multiply(&loan.amount, &(interest_loan as i128))?;
    let total_fees = divide(&fees_fixed_point, &(TOTAL_BASIS_PERCENTAGE as i128))?;

    Ok(total_fees)
}


fn calculate_percentage(amount: &i128, total_balance: &i128) -> Result<i64, LPError> {
    let divisor = multiply(amount, &(to_fixed_point()?))?;
    let percentage = divide(&divisor, total_balance)?;
    Ok(percentage as i64)
}

pub fn calculate_new_lender_amount(
    loan_amount: &i128,
    lender_balance: &i128,
    percentage: i64,
) -> Result<i128, LPError> {
    let amount_loaned = multiply(loan_amount, &(percentage as i128))?;
    let amount = divide(&amount_loaned, &(to_fixed_point()?))?;
    let lender_amount = subtract(lender_balance, &amount)?;
    Ok(lender_amount)
}

pub fn calculate_repayment_amount(amount: i128, percentage: i64) -> Result<i128, LPError> {
    let divisor = multiply(&amount, &(percentage as i128))?;
    let repayment_amount = divide(&divisor, &(to_fixed_point()?))?;
    Ok(repayment_amount)
}

pub fn process_lender_contribution(
    env: &Env,
    contributions: Vec<Address>,
    loan_amount: &i128,
    total_balance: &i128,
) -> ContributionResult {
    let mut lender_contributions = Map::new(env);

    for address in contributions.iter() {
        let mut lender = read_lender(env, &address)?;

        if lender.status == LenderStatus::Enabled {
            let percentage = calculate_percentage(&lender.balance, total_balance)?;
            let new_lender_amount =
                calculate_new_lender_amount(loan_amount, &lender.balance, percentage)?;

            lender.balance = new_lender_amount;
            lender.active_loans = sum(&lender.active_loans, &1)?;
            lender_contributions.set(address.clone(), percentage);

            write_lender(env, &address, &lender);
        }
    }
    Ok(lender_contributions)
}
