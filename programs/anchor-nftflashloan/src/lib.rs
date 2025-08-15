use anchor_lang::prelude::*;

declare_id!("H6jEh2UbEn4y4LzjYi8ZB4WP7nPUMFVmcpZJ8rbwQepa");

#[program]
pub mod anchor_nftflashloan {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
