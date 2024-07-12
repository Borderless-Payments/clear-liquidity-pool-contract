use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum LPError {
    AlreadyInitialized = 1,
    AmountMustBePositive = 2,
    AddressNotRegistered = 3,
    BalanceNotAvailableForAmountRequested = 4,
    BorrowerAlreadyRegistered = 5,
    BorrowerNotRegistered = 6,
    InsufficientBalance = 7,
    LoanNotFoundOrExists = 8,
    LenderAlreadyRegistered = 9,
    LenderNotRegistered = 10,
}
