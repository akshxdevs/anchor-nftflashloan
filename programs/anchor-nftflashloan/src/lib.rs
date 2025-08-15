use anchor_lang::prelude::*;

declare_id!("H6jEh2UbEn4y4LzjYi8ZB4WP7nPUMFVmcpZJ8rbwQepa");

#[program]
pub mod anchor_nftflashloan {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>,fees_bps:u16,merkle_root:Option<[u8;32]>) -> Result<()> {
        let config = &mut ctx.accounts.config;

        config.admin = ctx.accounts.admin.key();
        config.bump = ctx.bumps.config;
        config.paused = false;
        config.fee_bps = fees_bps;
        config.merkle_root = merkle_root;
        Ok(())
    }
    pub fn set_fee_bps(ctx: Context<OnlyAdminMutConfig>, new_fee_bps: u16) -> Result<()> {
        ctx.accounts.target.fee_bps = new_fee_bps;
        Ok(())
    }

    pub fn set_merkle_root(
        ctx: Context<OnlyAdminMutConfig>, 
        merkle_root: Option<[u8; 32]>
    ) -> Result<()> {
        ctx.accounts.target.merkle_root = merkle_root;
        Ok(())
    }

    pub fn set_paused(ctx: Context<OnlyAdminMutConfig>, paused: bool) -> Result<()> {
        ctx.accounts.target.paused = paused;
        Ok(())
    }
}

#[account]
pub struct Config{
    pub admin:Pubkey,
    pub fee_bps:u16,
    pub merkle_root:Option<[u8;32]>,
    pub bump:u8,
    pub paused:bool
}impl Config {
    pub const SIZE:usize = 32 + 2 + 33 + 4 + 4;
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        seeds = [b"config",admin.key().as_ref()],
        payer = admin,
        space = 8 + Config::SIZE,
        bump
    )]
    pub config:Account<'info,Config>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct OnlyAdminMut<'info, T: AccountSerialize + AccountDeserialize + Owner + Clone> {
    #[account(mut)]
    pub target: Account<'info, T>,
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
pub struct OnlyAdminMutConfig<'info> {
    #[account(mut)]
    pub target: Account<'info, Config>,
    pub admin: Signer<'info>,
}