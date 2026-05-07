#![allow(dead_code)]

use anchor_lang::{prelude::AccountInfo, Discriminator};
use anchor_spl::token_interface::spl_token_2022;
use bytemuck::{bytes_of, Pod};
use solana_program::{
    program_option::COption,
    program_pack::Pack,
    pubkey::Pubkey,
};

pub struct TestAccount {
    pub key: Pubkey,
    owner: Pubkey,
    lamports: u64,
    data: Vec<u8>,
    signer: bool,
    writable: bool,
    executable: bool,
}

impl TestAccount {
    pub fn new(key: Pubkey, owner: Pubkey) -> Self {
        Self {
            key,
            owner,
            lamports: 0,
            data: Vec::new(),
            signer: false,
            writable: false,
            executable: false,
        }
    }

    pub fn with_lamports(mut self, lamports: u64) -> Self {
        self.lamports = lamports;
        self
    }

    pub fn with_data(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self
    }

    pub fn signer(mut self) -> Self {
        self.signer = true;
        self
    }

    pub fn writable(mut self) -> Self {
        self.writable = true;
        self
    }

    pub fn executable(mut self) -> Self {
        self.executable = true;
        self
    }

    pub fn info(&mut self) -> AccountInfo<'_> {
        AccountInfo::new(
            &self.key,
            self.signer,
            self.writable,
            &mut self.lamports,
            &mut self.data,
            &self.owner,
            self.executable,
            0,
        )
    }
}

pub fn zero_copy_account_data<T: Discriminator + Pod>(account: &T) -> Vec<u8> {
    let mut data = vec![0; 8 + std::mem::size_of::<T>()];
    data[..8].copy_from_slice(&T::discriminator());
    data[8..].copy_from_slice(bytes_of(account));
    data
}

pub fn zeroed_zero_copy_account_data<T: Discriminator + Pod>() -> Vec<u8> {
    vec![0; 8 + std::mem::size_of::<T>()]
}

pub fn mint_account_data() -> Vec<u8> {
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

pub fn token_account_data(mint: Pubkey, owner: Pubkey, amount: u64) -> Vec<u8> {
    let mut data = vec![0; spl_token_2022::state::Account::LEN];
    let token_account = spl_token_2022::state::Account {
        mint,
        owner,
        amount,
        delegate: COption::None,
        state: spl_token_2022::state::AccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    };
    spl_token_2022::state::Account::pack(token_account, &mut data).unwrap();
    data
}
