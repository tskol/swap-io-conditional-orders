use std::num::TryFromIntError;

use anchor_lang::prelude::*;
use num_enum::TryFromPrimitive;
use thiserror::Error;

#[error_code]
#[derive(Error, PartialEq, Eq, TryFromPrimitive)]
pub enum OrdoError {
    #[msg("Express relay disabled")]
    ExpressRelayDisabled,

    #[msg("Invalid feed id")]
    InvalidFeedId,

    #[msg("Invalid BPS value, must be between 0 and 10000")]
    InvalidBps,

    #[msg("Invalid withdraw fee amount")]
    InvalidWithdrawFeeAmount,

    #[msg("TPSL not enabled")]
    TPSLNotEnabled,

    #[msg("TPSL min distance not met")]
    TPSLMinDistanceNotMet,

    #[msg("Price too high")]
    PriceTooHigh,

    #[msg("Output vault required")]
    OutputVaultRequired,

    #[msg("Order can't be canceled")]
    OrderCanNotBeCanceled,

    #[msg("Order not active")]
    OrderNotActive,

    #[msg("Invalid admin authority")]
    InvalidAdminAuthority,

    #[msg("Invalid pda authority")]
    InvalidPdaAuthority,

    #[msg("Invalid config option")]
    InvalidConfigOption,

    #[msg("Order owner account is not the order owner")]
    InvalidOrderOwner,

    #[msg("Out of range integral conversion attempted")]
    OutOfRangeIntegralConversion,

    #[msg("Invalid boolean flag, valid values are 0 and 1")]
    InvalidFlag,

    #[msg("Mathematical operation with overflow")]
    MathOverflow,

    #[msg("Order input amount invalid")]
    OrderInputAmountInvalid,

    #[msg("Order output amount invalid")]
    OrderOutputAmountInvalid,

    #[msg("Host fee bps must be between 0 and 10000")]
    InvalidHostFee,

    #[msg("Conversion between integers failed")]
    IntegerOverflow,

    #[msg("Tip balance less than accounted tip")]
    InvalidTipBalance,

    #[msg("Tip transfer amount is less than expected")]
    InvalidTipTransferAmount,

    #[msg("Host tup amount is less than accounted for")]
    InvalidHostTipBalance,

    #[msg("Order within flash operation - all otehr actions are blocked")]
    OrderWithinFlashOperation,

    #[msg("CPI not allowed")]
    CPINotAllowed,

    #[msg("Flash take_order is blocked")]
    FlashExecuteOrderBlocked,

    #[msg("Some unexpected instructions are present in the tx. Either before or after the flash ixs, or some ix target the same program between")]
    FlashTxWithUnexpectedIxs,

    #[msg("Flash ixs initiated without the closing ix in the transaction")]
    FlashIxsNotEnded,

    #[msg("Flash ixs ended without the starting ix in the transaction")]
    FlashIxsNotStarted,

    #[msg("Some accounts differ between the two flash ixs")]
    FlashIxsAccountMismatch,

    #[msg("Some args differ between the two flash ixs")]
    FlashIxsArgsMismatch,

    #[msg("Order is not within flash operation")]
    OrderNotWithinFlashOperation,

    #[msg("Emergency mode is enabled")]
    EmergencyModeEnabled,

    #[msg("Creating new ordersis blocked")]
    CreatingNewOrdersBlocked,

    #[msg("Orders taking is blocked")]
    OrderTakingBlocked,

    #[msg("Order input amount larger than the remaining")]
    OrderInputAmountTooLarge,

    #[msg("Permissionless order taking not enabled, please provide permission account")]
    PermissionRequiredPermissionlessNotEnabled,

    #[msg("Permission address does not match order address")]
    PermissionDoesNotMatchOrder,

    #[msg("Invalid ata address")]
    InvalidAtaAddress,

    #[msg("Maker output ata required when output mint is not WSOL")]
    MakerOutputAtaRequired,

    #[msg("Intermediary output token account required when output mint is WSOL")]
    IntermediaryOutputTokenAccountRequired,

    #[msg("Not enough balance for rent")]
    NotEnoughBalanceForRent,

    #[msg("Order can not be closed - Not enough time passed since last update")]
    NotEnoughTimePassedSinceLastUpdate,

    #[msg("Order input and output mints are the same")]
    OrderSameMint,

    #[msg("Mint has a token (2022) extension that is not supported")]
    UnsupportedTokenExtension,

    #[msg("Can't have an spl token mint with a t22 account")]
    InvalidTokenAccount,

    #[msg("The order type is invalid")]
    OrderTypeInvalid,

    #[msg("Token account is not initialized")]
    UninitializedTokenAccount,

    #[msg("Account is not owned by the token program")]
    InvalidTokenAccountOwner,

    #[msg("Account is not a valid token account")]
    InvalidAccount,

    #[msg("Token account has incorrect mint")]
    InvalidTokenMint,

    #[msg("Token account has incorrect authority")]
    InvalidTokenAuthority,

    #[msg("The provided parameter type is invalid")]
    InvalidParameterType,

    #[msg("The counterparty is not the taker")]
    CounterpartyDisallowed,

    #[msg("The swap input amount is larger than the maximum allowed")]
    SwapInputAmountTooLarge,

    #[msg("The swap output amount is smaller than the minimum allowed")]
    SwapOutputAmountTooSmall,

    #[msg("The swap input balance change is positive, expected negative")]
    SwapInputInvalidBalanceChange,

    #[msg("The swap output balance change is negative, expected positive")]
    SwapOutputInvalidBalanceChange,

    #[msg("The order parameters are invalid")]
    OrderParametersInvalid,

    #[msg("Order expired")]
    OrderExpired,
}

impl From<TryFromIntError> for OrdoError {
    fn from(_: TryFromIntError) -> OrdoError {
        OrdoError::OutOfRangeIntegralConversion
    }
}

pub type OrdoResult<T> = std::result::Result<T, OrdoError>;
