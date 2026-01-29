export const PROGRAM_ID = "6b2ieZ8k2t6yxD6wS24Zq5tb9QqtivfvXUKKntdByDEk";
export const PYTH_ENDPOINT = "https://hermes.pyth.network";

export const GLOBAL_CONFIG_SIZE = 2224;
export const ORDER_SIZE = 536;

export const GLOBAL_AUTH_SEED = "authority";
export const ESCROW_VAULT_SEED = "escrow_vault";
export const FEE_VAULT_SEED = "fee_vault";
export const ORACLE_POOL_SEED = "oracle_pool";

export const OrderStatus = {
    Active: 0,
    Filled: 1,
    Cancelled: 2,
};

export const OrderType = {
    Vanilla: 0,
    LimitParent: 1,
    LimitTP: 2,
    LimitSL: 3,
};

export type UpdateOrderMode = 
  | { updatePermissionless: {} }
  | { updateCounterparty: {} };

export const UpdateOrderMode = {
  UpdatePermissionless: { updatePermissionless: {} } as UpdateOrderMode,
  UpdateCounterparty: { updateCounterparty: {} } as UpdateOrderMode,
};

export const UPDATE_GLOBAL_CONFIG_BYTE_SIZE = 128;

export const UpdateGlobalConfigMode = {
    UpdateEmergencyMode: 0,
    UpdateFlashTakeOrderBlocked: 1,
    UpdateBlockNewOrders: 2,
    UpdateBlockOrderTaking: 3,
    UpdateHostFeeBps: 4,
    UpdateAdminAuthorityCached: 5,
    UpdateOrderCloseDelaySeconds: 6,
    UpdateTxnFeeCost: 7,
    UpdateAtaCreationCost: 8,
    UpdateTpSlEnabled: 9,
    UpdateCreateOrderFeeBps: 10,
    UpdateParentFillFeeKeeperBps: 11,
    UpdateParentFillFeeProtocolBps: 12,
    UpdateTpSlChildFeeKeeperBps: 13,
    UpdateTpSlChildFeeProtocolBps: 14,
    UpdateOracleMaxStalenessSeconds: 15,
    UpdateSlMaxUpwardDeviationBps: 16,
    UpdateTpSlMinDistanceBps: 17,
    UpdateAllowedTaker: 18,
}

export const LimoError = {
    InvalidFeedId: "Invalid feed id",
    InvalidBps: "Invalid BPS value, must be between 0 and 10000",
    InvalidWithdrawFeeAmount: "Invalid withdraw fee amount",
    TPSLNotEnabled: "TPSL not enabled",
    TPSLMinDistanceNotMet: "TPSL min distance not met",
    PriceTooHigh: "Price too high",
    OutputVaultRequired: "Output vault required",
    OrderCanNotBeCanceled: "Order can't be canceled",
    OrderNotActive: "Order not active",
    InvalidAdminAuthority: "Invalid admin authority",
    InvalidPdaAuthority: "Invalid pda authority",
    InvalidConfigOption: "Invalid config option",
    InvalidOrderOwner: "Order owner account is not the order owner",
    OutOfRangeIntegralConversion: "Out of range integral conversion attempted",
    InvalidFlag: "Invalid boolean flag, valid values are 0 and 1",
    MathOverflow: "Mathematical operation with overflow",
    OrderInputAmountInvalid: "Order input amount invalid",
    OrderOutputAmountInvalid: "Order output amount invalid",
    InvalidHostFee: "Host fee bps must be between 0 and 10000",
    IntegerOverflow: "Conversion between integers failed",
    InvalidTipBalance: "Tip balance less than accounted tip",
    InvalidTipTransferAmount: "Tip transfer amount is less than expected",
    InvalidHostTipBalance: "Host tup amount is less than accounted for",
    OrderWithinFlashOperation: "Order within flash operation - all otehr actions are blocked",
    CPINotAllowed: "CPI not allowed",
    FlashTakeOrderBlocked: "Flash take_order is blocked",
    FlashTxWithUnexpectedIxs: "Some unexpected instructions are present in the tx. Either before or after the flash ixs, or some ix target the same program between",
    FlashIxsNotEnded: "Flash ixs initiated without the closing ix in the transaction",
    FlashIxsNotStarted: "Flash ixs ended without the starting ix in the transaction",
    FlashIxsAccountMismatch: "Some accounts differ between the two flash ixs",
    FlashIxsArgsMismatch: "Some args differ between the two flash ixs",
    OrderNotWithinFlashOperation: "Order is not within flash operation",
    EmergencyModeEnabled: "Emergency mode is enabled",
    CreatingNewOrdersBlocked: "Creating new ordersis blocked",
    OrderTakingBlocked: "Orders taking is blocked",
    OrderInputAmountTooLarge: "Order input amount larger than the remaining",
    PermissionRequiredPermissionlessNotEnabled: "Permissionless order taking not enabled, please provide permission account",
    PermissionDoesNotMatchOrder: "Permission address does not match order address",
    InvalidAtaAddress: "Invalid ata address",
    MakerOutputAtaRequired: "Maker output ata required when output mint is not WSOL",
    IntermediaryOutputTokenAccountRequired: "Intermediary output token account required when output mint is WSOL",
    NotEnoughBalanceForRent: "Not enough balance for rent",
    NotEnoughTimePassedSinceLastUpdate: "Order can not be closed - Not enough time passed since last update",
    OrderSameMint: "Order input and output mints are the same",
    UnsupportedTokenExtension: "Mint has a token (2022) extension that is not supported",
    InvalidTokenAccount: "Can't have an spl token mint with a t22 account",
    InvalidTokenAccountOwner: "Account is not owned by the token program",
    InvalidAccount: "Account is not a valid token account",
    InvalidTokenMint: "Token account has incorrect mint",
    InvalidTokenAuthority: "Token account has incorrect authority",
    InvalidParameterType: "The provided parameter type is invalid",
    CounterpartyDisallowed: "The counterparty is not the taker",
    SwapInputAmountTooLarge: "The swap input amount is larger than the maximum allowed",
    SwapOutputAmountTooSmall: "The swap output amount is smaller than the minimum allowed",
    SwapInputInvalidBalanceChange: "The swap input balance change is positive, expected negative",
    SwapOutputInvalidBalanceChange: "The swap output balance change is negative, expected positive",
    OrderParametersInvalid: "The order parameters are invalid",
}