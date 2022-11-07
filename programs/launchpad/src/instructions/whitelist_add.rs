//! WhitelistAdd instruction handler

use {
    crate::{
        error::LaunchpadError,
        state::{self, auction::Auction, bid::Bid},
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct WhitelistAdd<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        has_one = owner,
        seeds = [b"auction", auction.common.name.as_bytes()],
        bump = auction.bump
    )]
    pub auction: Box<Account<'info, Auction>>,

    system_program: Program<'info, System>,
    // remaining accounts:
    //   Bid accounts for addresses to be whitelisted (write, unsigned)
    //     seeds = [b"bid", address, auction.key().as_ref()]
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct WhitelistAddParams {
    addresses: Vec<Pubkey>,
    bumps: Vec<u8>,
}

pub fn whitelist_add<'info>(
    ctx: Context<'_, '_, '_, 'info, WhitelistAdd<'info>>,
    params: &WhitelistAddParams,
) -> Result<()> {
    if params.addresses.is_empty()
        || ctx.remaining_accounts.len() != params.addresses.len()
        || params.addresses.len() != params.bumps.len()
    {
        return Err(ProgramError::NotEnoughAccountKeys.into());
    }

    let mut bid_accounts = state::create_bid_accounts(
        ctx.remaining_accounts,
        &params.addresses,
        &params.bumps,
        ctx.accounts.owner.to_account_info(),
        &ctx.accounts.auction.key(),
        ctx.accounts.system_program.to_account_info(),
    )?;
    for ((bid, owner), bump) in bid_accounts
        .iter_mut()
        .zip(params.addresses.iter())
        .zip(params.bumps.iter())
    {
        // validate bid address
        let expected_bid_key = Pubkey::create_program_address(
            &[
                b"bid",
                owner.as_ref(),
                ctx.accounts.auction.key().as_ref(),
                &[*bump],
            ],
            &crate::ID,
        )
        .map_err(|_| LaunchpadError::InvalidBidAddress)?;
        require_keys_eq!(
            bid.key(),
            expected_bid_key,
            LaunchpadError::InvalidBidAddress
        );

        // add to white-list
        if bid.bump == 0 {
            bid.owner = *owner;
            bid.auction = ctx.accounts.auction.key();
            bid.seller_initialized = true;
            bid.bump = *bump;
        }
        bid.whitelisted = true;
    }
    state::save_accounts(&bid_accounts)?;

    Ok(())
}
