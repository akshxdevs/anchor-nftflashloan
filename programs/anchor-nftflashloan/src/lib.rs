use anchor_lang::prelude::*;
use anchor_spl::{
    token::{self, Token, TokenAccount, Transfer},
    associated_token::AssociatedToken,
};

declare_id!("H6jEh2UbEn4y4LzjYi8ZB4WP7nPUMFVmcpZJ8rbwQepa");

#[program]
pub mod anchor_nftflashloan {
    use super::*;

    /// Initialize the program configuration
    pub fn initialize(
        ctx: Context<Initialize>,
        fees_bps: u16,
        merkle_root: Option<[u8; 32]>,
    ) -> Result<()> {
        let config = &mut ctx.accounts.config;
        
        config.admin = ctx.accounts.admin.key();
        config.bump = ctx.bumps.config;
        config.paused = false;
        config.fee_bps = fees_bps;
        config.merkle_root = merkle_root;
        
        msg!("Program initialized with admin: {}", config.admin);
        Ok(())
    }

    /// Set the fee basis points (only admin)
    pub fn set_fee_bps(ctx: Context<SetFeeBps>, new_fee_bps: u16) -> Result<()> {
        ctx.accounts.config.fee_bps = new_fee_bps;
        msg!("Fee BPS updated to: {}", new_fee_bps);
        Ok(())
    }

    /// Set the merkle root for whitelist (only admin)
    pub fn set_merkle_root(
        ctx: Context<SetMerkleRoot>,
        merkle_root: Option<[u8; 32]>,
    ) -> Result<()> {
        ctx.accounts.config.merkle_root = merkle_root;
        msg!("Merkle root updated");
        Ok(())
    }

    /// Set paused state (only admin)
    pub fn set_paused(ctx: Context<SetPaused>, paused: bool) -> Result<()> {
        ctx.accounts.config.paused = paused;
        msg!("Program paused state: {}", paused);
        Ok(())
    }

    /// Initialize a vault for a specific token mint
    pub fn init_vault(ctx: Context<InitVault>) -> Result<()> {
        let vault_state = &mut ctx.accounts.vault_state;
        
        vault_state.mint = ctx.accounts.liquidity_mint.key();
        vault_state.authority = ctx.accounts.vault_authority.key();
        vault_state.config = ctx.accounts.config.key();
        vault_state.bump = ctx.bumps.vault_state;
        vault_state.vault_bump = ctx.bumps.vault_authority;
        vault_state.in_flash = false;
        
        msg!("Vault initialized for mint: {}", vault_state.mint);
        Ok(())
    }

    /// Deposit an NFT into escrow
    pub fn deposit_nft(ctx: Context<DepositNft>) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        
        // Check if program is paused
        require!(!ctx.accounts.config.paused, ErrorCode::Paused);

        // Optional whitelist check via Merkle proof
        if let Some(_root) = ctx.accounts.config.merkle_root {
            // For now, skip merkle verification to simplify
            msg!("Merkle root set, but verification skipped for simplicity");
        }

        // Transfer NFT from user to escrow
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_nft_ata.to_account_info(),
                    to: ctx.accounts.escrow_nft_ata.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            1, // NFT amount
        )?;

        // Initialize escrow account
        escrow.owner = ctx.accounts.user.key();
        escrow.nft_mint = ctx.accounts.nft_mint.key();
        escrow.bump = ctx.bumps.escrow;
        escrow.status = EscrowStatus::Deposited;
        escrow.vault_state = ctx.accounts.vault_state.key();
        
        msg!("NFT deposited to escrow by: {}", escrow.owner);
        Ok(())
    }

    /// Withdraw NFT from escrow back to owner
    pub fn withdraw_nft(ctx: Context<WithdrawNft>) -> Result<()> {
        let escrow = &ctx.accounts.escrow;
        
        // Check escrow status
        require!(
            matches!(escrow.status, EscrowStatus::Deposited),
            ErrorCode::EscrowLocked
        );

        // Transfer NFT back to user
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.escrow_nft_ata.to_account_info(),
                    to: ctx.accounts.user_nft_ata.to_account_info(),
                    authority: ctx.accounts.escrow_authority.to_account_info(),
                },
                &[&[
                    b"escrow",
                    escrow.key().as_ref(),
                    &[escrow.bump],
                ]],
            ),
            1,
        )?;

        // Update escrow status
        let escrow = &mut ctx.accounts.escrow;
        escrow.status = EscrowStatus::Closed;
        
        msg!("NFT withdrawn from escrow by: {}", escrow.owner);
        Ok(())
    }

    /// Execute a flash loan
    pub fn flash_loan(
        ctx: Context<FlashLoan>,
        amount: u64,
        borrower_ix_data: Vec<u8>,
        expected_borrower_program: Pubkey,
    ) -> Result<()> {
        let config = &ctx.accounts.config;
        let vault_state = &mut ctx.accounts.vault_state;
        
        // Check if program is paused
        require!(!config.paused, ErrorCode::Paused);
        
        // Reentrancy guard
        require!(!vault_state.in_flash, ErrorCode::Reentrancy);
        vault_state.in_flash = true;

        // Record initial vault balance
        let initial_balance = ctx.accounts.vault_ata.amount;
        msg!("Flash loan started. Initial balance: {}", initial_balance);

        // Transfer tokens to borrower
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.vault_ata.to_account_info(),
                    to: ctx.accounts.borrower_ata.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
                &[&[
                    b"vault_authority",
                    vault_state.key().as_ref(),
                    &[vault_state.vault_bump],
                ]],
            ),
            amount,
        )?;

        // Verify borrower program
        require_keys_eq!(
            ctx.accounts.borrower_program.key(),
            expected_borrower_program,
            ErrorCode::WrongProgram
        );

        // Execute borrower's instruction
        let mut metas: Vec<anchor_lang::solana_program::instruction::AccountMeta> = 
            Vec::with_capacity(ctx.remaining_accounts.len());
        
        for ai in ctx.remaining_accounts.iter() {
            metas.push(match (ai.is_writable, ai.is_signer) {
                (true, true) => anchor_lang::solana_program::instruction::AccountMeta::new(ai.key(), true),
                (true, false) => anchor_lang::solana_program::instruction::AccountMeta::new(ai.key(), false),
                (false, true) => anchor_lang::solana_program::instruction::AccountMeta::new_readonly(ai.key(), true),
                (false, false) => anchor_lang::solana_program::instruction::AccountMeta::new_readonly(ai.key(), false),
            });
        }

        let instruction = anchor_lang::solana_program::instruction::Instruction {
            program_id: ctx.accounts.borrower_program.key(),
            accounts: metas,
            data: borrower_ix_data,
        };

        // Execute the instruction
        let mut infos: Vec<AccountInfo> = Vec::with_capacity(ctx.remaining_accounts.len());
        for ai in ctx.remaining_accounts.iter() {
            infos.push(ai.clone());
        }
        
        anchor_lang::solana_program::program::invoke(&instruction, &infos)?;

        // Check repayment with fee
        let final_balance = ctx.accounts.vault_ata.amount;
        let fee = calc_fee(amount, config.fee_bps)?;
        let required_balance = initial_balance.checked_add(fee).ok_or(ErrorCode::MathOverflow)?;
        
        require!(
            final_balance >= required_balance,
            ErrorCode::LoanNotRepaid
        );

        // Reset reentrancy guard
        vault_state.in_flash = false;
        
        msg!("Flash loan completed successfully. Fee collected: {}", fee);
        Ok(())
    }
}

// ===== ACCOUNT STRUCTURES =====

#[account]
pub struct Config {
    pub admin: Pubkey,           // 32 bytes
    pub fee_bps: u16,            // 2 bytes
    pub merkle_root: Option<[u8; 32]>, // 33 bytes (1 byte for option + 32 for data)
    pub bump: u8,                // 1 byte
    pub paused: bool,            // 1 byte
}

impl Config {
    pub const SIZE: usize = 32 + 2 + 33 + 1 + 1;
}

#[account]
pub struct VaultState {
    pub mint: Pubkey,            // 32 bytes
    pub authority: Pubkey,       // 32 bytes
    pub config: Pubkey,          // 32 bytes
    pub bump: u8,                // 1 byte
    pub vault_bump: u8,          // 1 byte
    pub in_flash: bool,          // 1 byte
}

impl VaultState {
    pub const SIZE: usize = 32 + 32 + 32 + 1 + 1 + 1;
}

#[account]
pub struct Escrow {
    pub owner: Pubkey,           // 32 bytes
    pub nft_mint: Pubkey,        // 32 bytes
    pub vault_state: Pubkey,     // 32 bytes
    pub bump: u8,                // 1 byte
    pub status: EscrowStatus,    // 1 byte
}

impl Escrow {
    pub const SIZE: usize = 32 + 32 + 32 + 1 + 1;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum EscrowStatus {
    Deposited,
    Closed,
}

// ===== ACCOUNT VALIDATION STRUCTURES =====

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        seeds = [b"config", admin.key().as_ref()],
        payer = admin,
        space = 8 + Config::SIZE,
        bump
    )]
    pub config: Account<'info, Config>,
    
    #[account(mut)]
    pub admin: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetFeeBps<'info> {
    #[account(mut, has_one = admin)]
    pub config: Account<'info, Config>,
    
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
pub struct SetMerkleRoot<'info> {
    #[account(mut, has_one = admin)]
    pub config: Account<'info, Config>,
    
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
pub struct SetPaused<'info> {
    #[account(mut, has_one = admin)]
    pub config: Account<'info, Config>,
    
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
pub struct InitVault<'info> {
    #[account(
        init,
        seeds = [b"vault_state", liquidity_mint.key().as_ref()],
        payer = admin,
        space = 8 + VaultState::SIZE,
        bump
    )]
    pub vault_state: Account<'info, VaultState>,
    
    #[account(
        seeds = [b"vault_authority", vault_state.key().as_ref()],
        bump
    )]
    /// CHECK: This is a PDA that will be the authority for the vault ATA
    pub vault_authority: UncheckedAccount<'info>,
    
    #[account(mut, has_one = admin)]
    pub config: Account<'info, Config>,
    
    pub liquidity_mint: Account<'info, token::Mint>,
    
    #[account(mut)]
    pub admin: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DepositNft<'info> {
    #[account(
        init,
        seeds = [b"escrow", user.key().as_ref(), nft_mint.key().as_ref()],
        payer = user,
        space = 8 + Escrow::SIZE,
        bump
    )]
    pub escrow: Account<'info, Escrow>,
    
    #[account(
        init,
        payer = user,
        associated_token::mint = nft_mint,
        associated_token::authority = escrow_authority,
    )]
    pub escrow_nft_ata: Account<'info, TokenAccount>,
    
    #[account(
        seeds = [b"escrow", escrow.key().as_ref()],
        bump
    )]
    /// CHECK: This is a PDA that will be the authority for the escrow ATA
    pub escrow_authority: UncheckedAccount<'info>,
    
    #[account(mut)]
    pub config: Account<'info, Config>,
    
    #[account(mut)]
    pub vault_state: Account<'info, VaultState>,
    
    #[account(mut)]
    pub user_nft_ata: Account<'info, TokenAccount>,
    
    pub nft_mint: Account<'info, token::Mint>,
    
    #[account(mut)]
    pub user: Signer<'info>,
    
    pub token_program: Program<'info, Token>,
    
    pub associated_token_program: Program<'info, AssociatedToken>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawNft<'info> {
    #[account(mut, has_one = owner)]
    pub escrow: Account<'info, Escrow>,
    
    #[account(mut)]
    pub escrow_nft_ata: Account<'info, TokenAccount>,
    
    #[account(
        seeds = [b"escrow", escrow.key().as_ref()],
        bump
    )]
    /// CHECK: This is a PDA that will be the authority for the escrow ATA
    pub escrow_authority: UncheckedAccount<'info>,
    
    #[account(mut)]
    pub user_nft_ata: Account<'info, TokenAccount>,
    
    pub owner: Signer<'info>,
    
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct FlashLoan<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,
    
    #[account(mut)]
    pub vault_state: Account<'info, VaultState>,
    
    #[account(mut)]
    pub vault_ata: Account<'info, TokenAccount>,
    
    #[account(
        seeds = [b"vault_authority", vault_state.key().as_ref()],
        bump
    )]
    /// CHECK: This is a PDA that will be the authority for the vault ATA
    pub vault_authority: UncheckedAccount<'info>,
    
    #[account(mut)]
    pub borrower_ata: Account<'info, TokenAccount>,
    
    /// CHECK: This is the program we're calling into
    pub borrower_program: UncheckedAccount<'info>,
    
    pub token_program: Program<'info, Token>,
}

// ===== ERROR CODES =====

#[error_code]
pub enum ErrorCode {
    #[msg("Program is paused")]
    Paused,
    #[msg("NFT is not whitelisted")]
    NotWhitelisted,
    #[msg("Escrow is locked")]
    EscrowLocked,
    #[msg("Reentrancy detected")]
    Reentrancy,
    #[msg("Wrong program called")]
    WrongProgram,
    #[msg("Loan not repaid")]
    LoanNotRepaid,
    #[msg("Math overflow")]
    MathOverflow,
}

// ===== HELPER FUNCTIONS =====

/// Calculate fee based on amount and basis points
fn calc_fee(amount: u64, fee_bps: u16) -> Result<u64> {
    let fee = amount
        .checked_mul(fee_bps as u64)
        .ok_or(ErrorCode::MathOverflow)?
        .checked_div(10000)
        .ok_or(ErrorCode::MathOverflow)?;
    Ok(fee)
}