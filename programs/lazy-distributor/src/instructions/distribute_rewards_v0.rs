use crate::error::ErrorCode;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::{
  associated_token::AssociatedToken,
  token::{self, Mint, Token, TokenAccount, Transfer},
};

#[derive(Accounts)]
pub struct DistributeRewardsV0<'info> {
  #[account(mut)]
  pub payer: Signer<'info>,
  #[account(
    has_one = rewards_mint,
    has_one = rewards_escrow
  )]
  pub lazy_distributor: Box<Account<'info, LazyDistributorV0>>,
  #[account(
    mut,
    has_one = lazy_distributor,
    constraint = recipient.current_rewards.iter().flatten().count() >= ((lazy_distributor.oracles.len() + 1) / 2)
  )]
  pub recipient: Box<Account<'info, RecipientV0>>,
  pub rewards_mint: Box<Account<'info, Mint>>,
  #[account(mut)]
  pub rewards_escrow: Box<Account<'info, TokenAccount>>,
  #[account(
    constraint = recipient_mint_account.mint == recipient.mint,
    constraint = recipient_mint_account.amount > 0,
    has_one = owner
  )]
  pub recipient_mint_account: Box<Account<'info, TokenAccount>>,
  /// CHECK: Just required for ATA
  pub owner: AccountInfo<'info>,
  #[account(
    init_if_needed,
    payer = payer,
    associated_token::mint = rewards_mint,
    associated_token::authority = owner,
  )]
  pub destination_account: Box<Account<'info, TokenAccount>>,
  pub associated_token_program: Program<'info, AssociatedToken>,
  pub system_program: Program<'info, System>,
  pub token_program: Program<'info, Token>,
  pub rent: Sysvar<'info, Rent>,
}

pub fn handler(ctx: Context<DistributeRewardsV0>) -> Result<()> {
  let seeds: &[&[&[u8]]] = &[&[
    b"lazy_distributor",
    ctx.accounts.lazy_distributor.rewards_mint.as_ref(),
    &[ctx.accounts.lazy_distributor.bump_seed],
  ]];
  let recipient = &mut ctx.accounts.recipient;
  let mut filtered: Vec<u64> = recipient
    .current_rewards
    .clone()
    .into_iter()
    .flatten()
    .collect();
  filtered.sort_unstable();
  let median_idx = filtered.len() / 2;
  let median = filtered[median_idx];

  recipient.current_rewards = vec![None; ctx.accounts.lazy_distributor.oracles.len()];
  let to_dist = median
    .checked_sub(recipient.total_rewards)
    .ok_or_else(|| error!(ErrorCode::ArithmeticError))?;
  recipient.total_rewards = median;

  token::transfer(
    CpiContext::new_with_signer(
      ctx.accounts.token_program.to_account_info().clone(),
      Transfer {
        from: ctx.accounts.rewards_escrow.to_account_info().clone(),
        to: ctx.accounts.destination_account.to_account_info().clone(),
        authority: ctx.accounts.lazy_distributor.to_account_info().clone(),
      },
      seeds,
    ),
    to_dist,
  )?;

  Ok(())
}
