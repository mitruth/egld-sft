multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::storage;

const COMMUNITY_TREASURE: u32 = 250;

//TODO: whitelist functionality
//TODO: check if smart contract is payable ?
//TODO: check how to call the buy method by another person from the website

#[multiversx_sc::module]
pub trait Operations: storage::Storage {
  #[only_user_account]
  #[payable("EGLD")]
  #[endpoint(buy)]
  fn buy(&self, amount_of_tokens: u32, token_id: u64) {
    require!(!self.collection_token_id().is_empty(), "Collection token not issued!");
    require!(!self.token_price_tag(token_id).is_empty(), "SFT token with such id doesn't exist");
    require!(
      amount_of_tokens > 0,
      "The number of tokens provided can't be less than 1!"
    );
    require!(
      self.token_price_tag(token_id).get().max_per_address >= amount_of_tokens,
      "The number of tokens has to be less than or equal the maximum per address"
    );

    let payment_amount = self.call_value().egld_value();
    let single_payment_amount = &payment_amount / amount_of_tokens;
    let price_tag = self.token_price_tag(token_id).get().price;

    require!(
        single_payment_amount == price_tag,
        "Invalid amount as payment. Check payment per one token and amount of tokens you want to buy."
    );

    let collection_token = self.collection_token_id().get();
    let caller = self.blockchain().get_caller();

    self.send()
      .direct_esdt(&caller, &collection_token, token_id, &BigUint::from(amount_of_tokens));

    let payment_nonce: u64 = 0;
    let payment_token = &EgldOrEsdtTokenIdentifier::egld();

    let owner = self.blockchain().get_owner_address();
    self.send()
      .direct(&owner, &payment_token, payment_nonce, &payment_amount);
  }

  #[only_owner]
  #[endpoint(buyCommunity)]
  fn buy_community(&self, token_id: u64) {
    require!(!self.collection_token_id().is_empty(), "Collection token not issued!");
    require!(!self.token_price_tag(token_id).is_empty(), "SFT token with such id doesn't exist");

    let collection_token = self.collection_token_id().get();
    let caller = self.blockchain().get_caller();

    self.send()
      .direct_esdt(&caller, &collection_token, token_id, &BigUint::from(COMMUNITY_TREASURE));
  }

  // As an owner, claim Smart Contract balance - temporary solution for royalities, the SC has to be payable to be able to get royalties
  #[only_owner]
  #[endpoint(claimScFunds)]
  fn claim_sc_funds(&self) {
      self.send().direct_egld(
          &self.blockchain().get_caller(),
          &self
              .blockchain()
              .get_sc_balance(&EgldOrEsdtTokenIdentifier::egld(), 0),
      );
  }

  #[view(getPrice)]
  fn get_price(&self, token_id: u64) -> BigUint {
    self.token_price_tag(token_id).get().price
  }

  #[view(getTokenDisplayName)]
  fn get_token_display_name(&self, token_id: u64) -> ManagedBuffer {
    self.token_price_tag(token_id).get().display_name
  }

  #[view(getMaxAmountPerAddress)]
  fn get_max_amount_per_address(&self, token_id: u64) -> BigUint {
    self.token_price_tag(token_id).get().max_per_address
  }

}
