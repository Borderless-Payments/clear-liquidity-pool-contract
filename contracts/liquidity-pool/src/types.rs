use soroban_sdk::{contracttype, Address, Map};

#[derive(Clone)]
#[contracttype]
pub struct Loan {
    pub amount: i128,
    pub start_time: u64,
    pub contributions: Map<Address, i64>,
}

#[derive(Clone, PartialEq, Debug)]
#[contracttype]
#[repr(u32)]
pub enum LenderStatus {
    Enabled,
    Disabled,
    PendingRemoval,
}

#[derive(Clone)]
#[contracttype]
pub struct Lender {
    pub status: LenderStatus,
    pub balance: i128,
    pub active_loans: u32,
}

#[derive(Clone)]
#[contracttype]
pub struct Borrower {
    pub active: bool,
    pub min_withdraw: i128,
    pub max_withdraw: i128,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    TotalBalance,
    Token,
    Admin,
    Contribution,
    Vault,
    Borrower(Address),
    Lender(Address),
    Loan(Address),
}
