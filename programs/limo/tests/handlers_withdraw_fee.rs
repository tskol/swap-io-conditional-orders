use anchor_lang::{
    prelude::{
        AccountInfo, AccountLoader, Context, Interface, InterfaceAccount, Program, Pubkey, Signer,
        System,
    },
    Discriminator,
};
use anchor_spl::token_interface::{spl_token_2022, Mint, TokenAccount, TokenInterface};
use bytemuck::{bytes_of, Pod, Zeroable};
use limo::{
    handlers::withdraw_fee::{withdraw_fee, WithdrawFee, WithdrawFeeBumps},
    state::GlobalConfig,
};
use solana_program::{program_option::COption, program_pack::Pack};

fn zero_copy_account_data<T: Discriminator + Pod>(account: &T) -> Vec<u8> {
    let mut data = vec![0; 8 + std::mem::size_of::<T>()];
    data[..8].copy_from_slice(&T::discriminator());
    data[8..].copy_from_slice(bytes_of(account));
    data
}

fn mint_account_data() -> Vec<u8> {
    let mut data = vec![0; spl_token_2022::state::Mint::LEN];
    let mint = spl_token_2022::state::Mint {
        mint_authority: COption::None,
        supply: 0,
        decimals: 6,
        is_initialized: true,
        freeze_authority: COption::None,
    };
    spl_token_2022::state::Mint::pack(mint, &mut data).unwrap();
    data
}

fn token_account_data(mint: Pubkey, owner: Pubkey) -> Vec<u8> {
    let mut data = vec![0; spl_token_2022::state::Account::LEN];
    let token_account = spl_token_2022::state::Account {
        mint,
        owner,
        amount: 0,
        delegate: COption::None,
        state: spl_token_2022::state::AccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    };
    spl_token_2022::state::Account::pack(token_account, &mut data).unwrap();
    data
}

#[test]
fn withdraw_fee_handler_rejects_zero_amount_before_cpi() {
    let program_id = limo::ID;
    let owner = limo::ID;
    let admin_key = Pubkey::new_unique();
    let fee_receiver_key = Pubkey::new_unique();
    let global_config_key = Pubkey::new_unique();
    let pda_authority_key = Pubkey::new_unique();
    let token_mint_key = Pubkey::new_unique();
    let fee_receiver_ata_key = Pubkey::new_unique();
    let fee_vault_key = Pubkey::new_unique();
    let token_program_key = spl_token_2022::ID;
    let system_program_key = anchor_lang::system_program::ID;

    let mut admin_lamports = 0;
    let mut admin_data = [];
    let admin_info = AccountInfo::new(
        &admin_key,
        true,
        true,
        &mut admin_lamports,
        &mut admin_data,
        &owner,
        false,
        0,
    );

    let mut fee_receiver_lamports = 0;
    let mut fee_receiver_data = [];
    let fee_receiver_info = AccountInfo::new(
        &fee_receiver_key,
        false,
        true,
        &mut fee_receiver_lamports,
        &mut fee_receiver_data,
        &owner,
        false,
        0,
    );

    let mut global_config = GlobalConfig::zeroed();
    global_config.admin_authority = admin_key;
    global_config.pda_authority = pda_authority_key;
    global_config.pda_authority_bump = 254;
    let mut global_config_lamports = 0;
    let mut global_config_data = zero_copy_account_data(&global_config);
    let global_config_info = AccountInfo::new(
        &global_config_key,
        false,
        true,
        &mut global_config_lamports,
        &mut global_config_data,
        &owner,
        false,
        0,
    );

    let mut pda_authority_lamports = 0;
    let mut pda_authority_data = [];
    let pda_authority_info = AccountInfo::new(
        &pda_authority_key,
        false,
        false,
        &mut pda_authority_lamports,
        &mut pda_authority_data,
        &owner,
        false,
        0,
    );

    let mut token_mint_lamports = 1;
    let mut token_mint_data = mint_account_data();
    let token_mint_info = AccountInfo::new(
        &token_mint_key,
        false,
        false,
        &mut token_mint_lamports,
        &mut token_mint_data,
        &token_program_key,
        false,
        0,
    );

    let mut fee_receiver_ata_lamports = 1;
    let mut fee_receiver_ata_data = token_account_data(token_mint_key, fee_receiver_key);
    let fee_receiver_ata_info = AccountInfo::new(
        &fee_receiver_ata_key,
        false,
        true,
        &mut fee_receiver_ata_lamports,
        &mut fee_receiver_ata_data,
        &token_program_key,
        false,
        0,
    );

    let mut fee_vault_lamports = 1;
    let mut fee_vault_data = token_account_data(token_mint_key, pda_authority_key);
    let fee_vault_info = AccountInfo::new(
        &fee_vault_key,
        false,
        true,
        &mut fee_vault_lamports,
        &mut fee_vault_data,
        &token_program_key,
        false,
        0,
    );

    let mut token_program_lamports = 0;
    let mut token_program_data = [];
    let token_program_info = AccountInfo::new(
        &token_program_key,
        false,
        false,
        &mut token_program_lamports,
        &mut token_program_data,
        &owner,
        true,
        0,
    );

    let mut system_program_lamports = 0;
    let mut system_program_data = [];
    let system_program_info = AccountInfo::new(
        &system_program_key,
        false,
        false,
        &mut system_program_lamports,
        &mut system_program_data,
        &owner,
        true,
        0,
    );

    let mut accounts = WithdrawFee {
        admin_authority: Signer::try_from(&admin_info).unwrap(),
        fee_receiver: fee_receiver_info,
        fee_receiver_ata: Box::new(
            InterfaceAccount::<TokenAccount>::try_from(&fee_receiver_ata_info).unwrap(),
        ),
        global_config: AccountLoader::try_from(&global_config_info).unwrap(),
        pda_authority: pda_authority_info,
        token_mint: Box::new(InterfaceAccount::<Mint>::try_from(&token_mint_info).unwrap()),
        fee_vault: Box::new(InterfaceAccount::<TokenAccount>::try_from(&fee_vault_info).unwrap()),
        token_program: Interface::<TokenInterface>::try_from(&token_program_info).unwrap(),
        system_program: Program::<System>::try_from(&system_program_info).unwrap(),
    };
    let ctx = Context::new(
        &program_id,
        &mut accounts,
        &[],
        WithdrawFeeBumps { fee_vault: 254 },
    );

    assert!(withdraw_fee(ctx, 0).is_err());
}
