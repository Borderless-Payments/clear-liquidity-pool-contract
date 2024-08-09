#![cfg(test)]

use crate::errors::LPError;
use crate::storage::{
    has_borrower, has_lender, read_admin, read_borrower, read_contract_balance, read_contributions,
    read_lender, read_loan, read_loans, read_token, read_vault,
};
use crate::types::{Borrower, LenderStatus};
use crate::LiquidityPoolContractClient;
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token::{self, StellarAssetClient},
    vec, Address, Env, Val, Vec,
};

pub fn set_timestamp_for_20_days(env: &Env) {
    let initial_timestamp = env.ledger().timestamp();
    let days = 20;
    let seconds_per_day = 86400;
    let new_timestamp = initial_timestamp + (days * seconds_per_day) as u64;

    env.ledger().set_timestamp(new_timestamp)
}

pub fn create_test_contract(
    env: &Env,
    admin: &Address,
    token: &Address,
    vault: &Address,
) -> (Address, LiquidityPoolContract) {
    let contract_id = register_test_contract(env);
    let contract = LiquidityPoolContract::new(env, contract_id.clone());

    contract.client().initialize(admin, token, vault);

    (contract_id, contract)
}

pub fn register_test_contract(env: &Env) -> Address {
    env.register_contract(None, crate::LiquidityPoolContract {})
}

pub fn create_token_contract<'a>(
    env: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract_address = env.register_stellar_asset_contract(admin.clone());
    (
        token::Client::new(env, &contract_address),
        token::StellarAssetClient::new(env, &contract_address),
    )
}

pub struct Setup<'a> {
    pub env: Env,
    pub admin: Address,
    pub vault: Address,
    pub token: token::Client<'a>,
    pub token_admin: StellarAssetClient<'a>,
    pub liquid_contract: LiquidityPoolContract,
    pub liquid_contract_id: Address,
}

pub struct LiquidityPoolContract {
    env: Env,
    contract_id: Address,
}

impl Setup<'_> {
    pub fn new() -> Self {
        let env = Env::default();
        let admin = Address::generate(&env);
        let vault = Address::generate(&env);
        let token_admin = Address::generate(&env);

        let (token, token_admin) = create_token_contract(&env, &token_admin);

        let (liquid_contract_id, liquid_contract) =
            create_test_contract(&env, &admin, &token.address, &vault);

        Self {
            env,
            admin,
            vault,
            token,
            token_admin,
            liquid_contract,
            liquid_contract_id,
        }
    }
}

impl LiquidityPoolContract {
    #[must_use]
    pub fn client(&self) -> LiquidityPoolContractClient {
        LiquidityPoolContractClient::new(&self.env, &self.contract_id)
    }

    #[must_use]
    pub fn new(env: &Env, contract_id: Address) -> Self {
        Self {
            env: env.clone(),
            contract_id,
        }
    }

    pub fn get_contract_events(&self) -> Vec<(Address, Vec<Val>, Val)> {
        let mut contract_events = vec![&self.env];

        self.env
            .events()
            .all()
            .iter()
            .filter(|event| event.0 == self.contract_id)
            .for_each(|event| contract_events.push_back(event));

        contract_events
    }

    pub fn read_admin(&self) -> Result<Address, LPError> {
        self.env.as_contract(&self.contract_id, || {
            let admin = read_admin(&self.env)?;
            Ok(admin)
        })
    }

    pub fn has_borrower(&self, borrower: &Address) -> bool {
        self.env
            .as_contract(&self.contract_id, || has_borrower(&self.env, borrower))
    }

    pub fn has_loan(&self, address: &Address, loan_id: u64) -> bool {
        self.env.as_contract(&self.contract_id, || {
            let loans = read_loans(&self.env, address);
            loans.contains_key(loan_id)
        })
    }

    pub fn has_lender(&self, lender: &Address) -> bool {
        self.env
            .as_contract(&self.contract_id, || has_lender(&self.env, lender))
    }

    pub fn read_token(&self) -> Result<Address, LPError> {
        self.env.as_contract(&self.contract_id, || {
            let token = read_token(&self.env)?;
            Ok(token)
        })
    }

    pub fn read_vault(&self) -> Result<Address, LPError> {
        self.env.as_contract(&self.contract_id, || {
            let vault = read_vault(&self.env)?;
            Ok(vault)
        })
    }

    pub fn read_borrower(&self, borrower: &Address) -> Result<Borrower, LPError> {
        self.env.as_contract(&self.contract_id, || {
            let borrower = read_borrower(&self.env, borrower)?;
            Ok(borrower)
        })
    }

    pub fn read_contract_balance(&self) -> i128 {
        self.env
            .as_contract(&self.contract_id, || read_contract_balance(&self.env))
    }

    pub fn read_loan_amount(&self, address: &Address, loan_id: u64) -> Result<i128, LPError> {
        self.env.as_contract(&self.contract_id, || {
            let loans = read_loans(&self.env, address);
            let loan = read_loan(&loans, &loan_id)?;
            Ok(loan.amount)
        })
    }

    pub fn read_lender(&self, lender: &Address) -> Result<i128, LPError> {
        self.env.as_contract(&self.contract_id, || {
            let lender = read_lender(&self.env, lender)?;
            Ok(lender.balance)
        })
    }

    pub fn read_lender_status(&self, lender: &Address) -> Result<LenderStatus, LPError> {
        self.env.as_contract(&self.contract_id, || {
            let lender = read_lender(&self.env, lender)?;
            Ok(lender.status)
        })
    }

    pub fn is_lender_in_contributions(&self, lender: &Address) -> bool {
        self.env.as_contract(&self.contract_id, || {
            let contributions = read_contributions(&self.env);
            contributions.iter().any(|address| address == *lender)
        })
    }
}
