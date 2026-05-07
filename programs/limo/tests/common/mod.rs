#![allow(dead_code)]

use anchor_lang::{prelude::AccountInfo, Discriminator};
use anchor_spl::token_interface::spl_token_2022;
use bytemuck::{bytes_of, Pod};
use solana_program::{
    clock::Clock,
    entrypoint::ProgramResult,
    instruction::Instruction,
    program_option::COption,
    program_pack::Pack,
    program_stubs::{set_syscall_stubs, SyscallStubs},
    pubkey::Pubkey,
    rent::Rent,
    sysvar::instructions::{
        construct_instructions_data, store_current_index, BorrowedAccountMeta, BorrowedInstruction,
    },
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

pub fn mint_account(key: Pubkey, token_program: Pubkey) -> TestAccount {
    TestAccount::new(key, token_program)
        .with_lamports(1)
        .with_data(mint_account_data())
}

pub fn token_account(
    key: Pubkey,
    token_program: Pubkey,
    mint: Pubkey,
    owner: Pubkey,
    amount: u64,
) -> TestAccount {
    TestAccount::new(key, token_program)
        .with_lamports(1)
        .with_data(token_account_data(mint, owner, amount))
        .writable()
}

pub fn instructions_sysvar_data(instructions: &[Instruction], current_index: u16) -> Vec<u8> {
    let borrowed_instructions = instructions
        .iter()
        .map(|instruction| BorrowedInstruction {
            program_id: &instruction.program_id,
            accounts: instruction
                .accounts
                .iter()
                .map(|account| BorrowedAccountMeta {
                    pubkey: &account.pubkey,
                    is_signer: account.is_signer,
                    is_writable: account.is_writable,
                })
                .collect(),
            data: &instruction.data,
        })
        .collect::<Vec<_>>();
    let mut data = construct_instructions_data(&borrowed_instructions);
    store_current_index(&mut data, current_index);
    data
}

pub fn executable_account(key: Pubkey, owner: Pubkey) -> TestAccount {
    TestAccount::new(key, owner).executable()
}

pub fn install_noop_syscall_stubs() {
    set_syscall_stubs(Box::new(NoopSyscallStubs {
        clock: Clock {
            slot: 0,
            epoch_start_timestamp: 0,
            epoch: 0,
            leader_schedule_epoch: 0,
            unix_timestamp: 100,
        },
        rent: Rent::default(),
    }));
}

struct NoopSyscallStubs {
    clock: Clock,
    rent: Rent,
}

impl SyscallStubs for NoopSyscallStubs {
    fn sol_invoke_signed(
        &self,
        _instruction: &Instruction,
        _account_infos: &[AccountInfo],
        _signers_seeds: &[&[&[u8]]],
    ) -> ProgramResult {
        Ok(())
    }

    fn sol_get_clock_sysvar(&self, var_addr: *mut u8) -> u64 {
        copy_sysvar(&self.clock, var_addr);
        0
    }

    fn sol_get_rent_sysvar(&self, var_addr: *mut u8) -> u64 {
        copy_sysvar(&self.rent, var_addr);
        0
    }
}

fn copy_sysvar<T>(sysvar: &T, var_addr: *mut u8) {
    unsafe {
        std::ptr::copy_nonoverlapping(
            sysvar as *const T as *const u8,
            var_addr,
            std::mem::size_of::<T>(),
        );
    }
}
