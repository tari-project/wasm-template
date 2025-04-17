//   Copyright 2024. The Tari Project
//
//   Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
//   following conditions are met:
//
//   1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
//   disclaimer.
//
//   2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
//   following disclaimer in the documentation and/or other materials provided with the distribution.
//
//   3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
//   products derived from this software without specific prior written permission.
//
//   THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
//   INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
//   DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
//   SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
//   SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
//   WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
//   USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use tari_template_lib::prelude::*;
use tari_template_lib::Hash;

/// TODO: create constant in template_lib for account template address (and other builtin templates)
pub const ACCOUNT_TEMPLATE_ADDRESS: Hash = Hash::from_array([0u8; 32]);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Bid {
    bidder_account: ComponentAddress,
    vault: Vault,
}

#[template]
mod nft_marketplace {
    use super::*;

    /// Simple English-like auctions
    /// The winner needs to claim the nft after the bidding period finishes. For simplicity, no marketplace fees are
    /// considered. There exist a lot more approaches to auctions, we can highlight:
    ///     - Price descending, dutch-like auctions. The first bidder gets the nft right away, no need to wait or claim
    ///       afterwards
    ///     - Blind auctions, were bids are not known until the end. This requires cryptography support, and implies that
    ///       all bidder's funds will be locked until the end of the auction
    pub struct Auction {
        seller_badge_resource: ResourceAddress,

        // The NFT will be locked, so the user gives away control to the marketplace
        // There are other approaches to this, like just allowing the seller to complete and confirm the bid at the end
        vault: Vault,

        // address of the account component of the seller
        seller_address: ComponentAddress,

        // minimum required price for a bid
        min_price: Option<Amount>,

        // price at which the NFT will be sold automatically
        buy_price: Option<Amount>,

        // Holds the current highest bidder, it's replaced when a new highest bidder appears
        highest_bid: Option<Bid>,

        // Time sensitive logic is a big issue, we need custom support for it. I see two options:
        //      1. Ad hoc protocol in the second layer to agree on timestamps (inside of a commitee? globally?)
        //      2. Leverage the base layer block number (~3 minute intervals)
        //      3. Use the current epoch (~30 min intervals)
        // We are going with (3) here. But either way this means custom utils and that some external state influences
        // execution
        ending_epoch: u64,
    }

    impl Auction {
        // returns a badge used to cancel the sell order in the future
        // the badge will contain immutable metadata referencing the nft being sold
        pub fn new(
            nft_bucket: Bucket,
            seller_address: ComponentAddress,
            min_price: Option<Amount>,
            buy_price: Option<Amount>,
            epoch_period: u64,
        ) -> (Component<Auction>, Bucket) {
            assert!(
                nft_bucket.resource_type() == ResourceType::NonFungible,
                "The resource is not a NFT"
            );

            assert!(
                nft_bucket.amount() == Amount(1),
                "Can only start an auction of a single NFT"
            );

            assert!(epoch_period > 0, "Invalid auction period");

            // needed to ensure that we can process the auction payments when it ends
            Self::assert_component_is_account(seller_address);

            // create the bucket with the badge to allow the seller to cancel the auction at any time
            // we make sure that only the initial badge will be minted
            let seller_badge_bucket = ResourceBuilder::non_fungible()
                .mintable(AccessRule::DenyAll)
                .burnable(AccessRule::AllowAll)
                .initial_supply_with_data(Some((NonFungibleId::random(), (&(), &()))));
            let seller_badge_resource = seller_badge_bucket.resource_address();

            // initialize the auction component
            let component = Component::new(Self {
                vault: Vault::from_bucket(nft_bucket),
                seller_address,
                min_price,
                buy_price,
                highest_bid: None,
                ending_epoch: Consensus::current_epoch() + epoch_period,
                seller_badge_resource,
            })
            .with_access_rules(AccessRules::allow_all())
            .create();

            (component, seller_badge_bucket)
        }

        // process a new bid for an ongoing auction
        pub fn bid(&mut self, bidder_account_address: ComponentAddress, payment: Bucket) {
            assert!(
                Consensus::current_epoch() < self.ending_epoch,
                "Auction has expired"
            );

            assert_eq!(
                payment.resource_address(),
                XTR,
                "Invalid payment resource, the marketplace only accepts Tari (XTR) tokens"
            );

            // validate that the bidder account is really an account
            // so we can deposit the refund later if a higher bidder comes
            // otherwise an attacker could block newer higher bids
            Self::assert_component_is_account(bidder_account_address);

            // check that the minimum price (if set) is met
            let payment_amount = payment.amount();
            if let Some(min_price) = self.min_price {
                assert!(payment_amount >= min_price, "Minimum price not met");
            }

            // immediatly refund the previous highest bidder if there is one
            if let Some(highest_bid) = &mut self.highest_bid {
                assert!(
                    payment_amount > highest_bid.vault.balance(),
                    "There is a higher bid placed"
                );
                let previous_bidder_account = ComponentManager::get(highest_bid.bidder_account);
                let refund_bucket = highest_bid.vault.withdraw_all();
                // TODO: improve call method generics when there is no return value
                previous_bidder_account.call::<_, ()>("deposit".to_string(), args![refund_bucket]);

                // update the highest bidder in the auction
                highest_bid.bidder_account = bidder_account_address;
                highest_bid.vault.deposit(payment.clone());
            } else {
                // the bidder is the first one to place a bid
                let highest_bid = Bid {
                    bidder_account: bidder_account_address,
                    vault: Vault::from_bucket(payment.clone()),
                };
                self.highest_bid = Some(highest_bid);
            }

            // if the bid meets the buying price, we process the sell immediatly
            if let Some(buy_price) = self.buy_price {
                assert!(
                    payment_amount <= buy_price,
                    "Payment exceeds the buying price"
                );
                if payment_amount == buy_price {
                    self.process_payments();
                }
            }
        }

        // finish the auction by sending the NFT and payment to the respective accounts
        // used by a bid seller to receive the bid payment, or by the buyer to get the NFT, whatever happens first
        pub fn finish(&mut self) {
            assert!(
                Consensus::current_epoch() >= self.ending_epoch,
                "Auction is still in progress"
            );

            self.process_payments();
        }

        // the seller wants to cancel the auction
        pub fn cancel(&mut self, seller_badge_bucket: Bucket) {
            // as the seller badge resource cannot be minted and only one token exist,
            // we only need to check that the resource address matches
            assert!(
                seller_badge_bucket.resource_address() == self.seller_badge_resource,
                "Invalid seller badge"
            );

            // an auction cannot be cancelled if it has ended
            assert!(
                Consensus::current_epoch() < self.ending_epoch,
                "Auction has ended"
            );

            // we are canceling the bid
            // so we need to pay back the highest bidded (if there's one)
            if let Some(highest_bid) = &mut self.highest_bid {
                let bidder_account = ComponentManager::get(highest_bid.bidder_account);
                let refund_bucket = highest_bid.vault.withdraw_all();
                bidder_account.call::<_, ()>("deposit".to_string(), args![refund_bucket]);
                // TODO: removing the bid ends up in a OrphanedSubstate error in the
                //       but we need to mark that the auction is finished somehow to prevent new bids
                // self.highest_bid = None;
            }

            // burn the seller token to prevent it from being used again, as it has no more purpose
            seller_badge_bucket.burn();

            // send the NFT back to the seller
            let seller_account = ComponentManager::get(self.seller_address);
            let nft_bucket = self.vault.withdraw_all();
            seller_account.call::<_, ()>("deposit".to_string(), args![nft_bucket]);
        }

        fn assert_component_is_account(component_address: ComponentAddress) {
            let component = ComponentManager::get(component_address);
            assert!(
                component.get_template_address() == ACCOUNT_TEMPLATE_ADDRESS,
                "Invalid bidder account"
            );
        }

        // this method MUST ALWAYS be private, to prevent auction cancellation by unauthorized third parties
        fn process_payments(&mut self) {
            let seller_account = ComponentManager::get(self.seller_address);
            let nft_bucket = self.vault.withdraw_all();

            if let Some(highest_bid) = &mut self.highest_bid {
                // deposit the nft to the bidder
                let bidder_account = ComponentManager::get(highest_bid.bidder_account);
                bidder_account.call::<_, ()>("deposit".to_string(), args![nft_bucket]);

                // deposit the funds to the seller
                let payment = highest_bid.vault.withdraw_all();
                seller_account.call::<_, ()>("deposit".to_string(), args![payment]);
            } else {
                // no bidders in the auction, so just return the NFT to the seller
                seller_account.call::<_, ()>("deposit".to_string(), args![nft_bucket]);
            }

            // TODO: burn the seller badge to avoid it being used again (should we recall it first?)
        }
    }
}
