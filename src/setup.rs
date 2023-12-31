multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::storage;
use crate::storage::TokenPriceTag;

const ROYALTIES_MAX: u32 = 10_000;
const METADATA_KEY_NAME: &[u8] = "metadata:".as_bytes();
const ATTR_SEPARATOR: &[u8] = ";".as_bytes();
const URI_SLASH: &[u8] = "/".as_bytes();
const TAGS_KEY_NAME: &[u8] = "tags:".as_bytes();

#[multiversx_sc::module]
pub trait Setup: storage::Storage {
    // Issue main collection token/handler
    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(issueToken)]
    fn issue_token(
        &self,
        collection_token_name: ManagedBuffer,
        collection_token_ticker: ManagedBuffer,
    ) {
        let issue_cost = self.call_value().egld_value();
        require!(self.collection_token_id().is_empty(), "Token already issued!");

        self.collection_token_name().set(&collection_token_name);

        self.send()
            .esdt_system_sc_proxy()
            .issue_semi_fungible(
                issue_cost,
                &collection_token_name,
                &collection_token_ticker,
                SemiFungibleTokenProperties {
                    can_freeze: false,
                    can_wipe: false,
                    can_pause: false,
                    can_transfer_create_role: false,
                    can_change_owner: false,
                    can_upgrade: false,
                    can_add_special_roles: true,
                },
            )
            .async_call()
            .with_callback(self.callbacks().issue_callback())
            .call_and_exit();
    }

    // Issue callback: Set the token id in storage or return founds when error
    #[callback]
    fn issue_callback(
        &self,
        #[call_result] result: ManagedAsyncCallResult<EgldOrEsdtTokenIdentifier>,
    ) {
        match result {
            ManagedAsyncCallResult::Ok(token_id) => {
                self.collection_token_id().set(&token_id.unwrap_esdt());
            }
            ManagedAsyncCallResult::Err(_) => {
                let caller = self.blockchain().get_owner_address();
                let returned = self.call_value().egld_or_single_esdt();
                if returned.token_identifier.is_egld() && returned.amount > 0 {
                    self.send()
                        .direct(&caller, &returned.token_identifier, 0, &returned.amount);
                }
            }
        }
    }

    // Set roles for the SFT token
    #[only_owner]
    #[endpoint(setLocalRoles)]
    fn set_local_roles(&self) {
        require!(!self.collection_token_id().is_empty(), "Token not issued!");

        self.send()
            .esdt_system_sc_proxy()
            .set_special_roles(
                &self.blockchain().get_sc_address(),
                &self.collection_token_id().get(),
                (&[
                    EsdtLocalRole::NftCreate,
                    EsdtLocalRole::NftAddQuantity,
                    EsdtLocalRole::NftBurn
                ][..])
                    .into_iter()
                    .cloned(),
            )
            .async_call()
            .call_and_exit();
    }

    // Create actual SFT with amount, assets etc. 
    #[only_owner]
    #[endpoint(createToken)]
    fn create_token(
        &self, 
        name: ManagedBuffer,
        selling_price: BigUint,
        metadata_ipfs_cid: ManagedBuffer,
        metadata_ipfs_file: ManagedBuffer,
        amount_of_tokens: BigUint,
        max_per_address: BigUint,
        royalties: BigUint,
        tags: ManagedBuffer,
        uris: MultiValueEncoded<ManagedBuffer>
    ) {
        require!(royalties <= ROYALTIES_MAX, "Royalties cannot exceed 100%!");
        require!(
            amount_of_tokens >= 1,
            "Amount of tokens should be at least 1!"
        );
        require!(selling_price >= 0, "Selling price can not be less than 0!");

        require!(!self.collection_token_id().is_empty(), "Token not issued!");

        let metadata_key_name = ManagedBuffer::new_from_bytes(METADATA_KEY_NAME);
        let tags_key_name = ManagedBuffer::new_from_bytes(TAGS_KEY_NAME);
        let separator = ManagedBuffer::new_from_bytes(ATTR_SEPARATOR);
        let metadata_slash = ManagedBuffer::new_from_bytes(URI_SLASH);

        let mut attributes = ManagedBuffer::new();
        attributes.append(&tags_key_name);
        attributes.append(&tags);
        attributes.append(&separator);
        attributes.append(&metadata_key_name);
        attributes.append(&metadata_ipfs_cid);
        attributes.append(&metadata_slash);
        attributes.append(&metadata_ipfs_file);

        let hash_buffer = self.crypto().sha256(&attributes);
        let attributes_hash = hash_buffer.as_managed_buffer();

        let token_id = self.collection_token_id().get();

        let uris_vec = uris.into_vec_of_buffers();

        let nonce = self.send().esdt_nft_create(&token_id, &amount_of_tokens, &name, &royalties, &attributes_hash, &attributes, &uris_vec);

        self.token_price_tag(nonce).set(TokenPriceTag {
          display_name: name,
          nonce,
          price: selling_price,
          max_per_address
        });
    }
}
