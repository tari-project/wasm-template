use tari_template_lib::prelude::*;
use std::collections::BTreeMap;

#[template]
mod nft_marketplace_index {
    use super::*;

    pub struct AuctionIndex {
        auction_template: TemplateAddress,
        auctions: BTreeMap<u64, Vec<ComponentAddress>>,
    }

    impl AuctionIndex {
        pub fn new(auction_template: TemplateAddress) -> Self {
            Self {
                auction_template,
                auctions: BTreeMap::new()
            }
        }

        // convenience method for external APIs and interfaces
        // TODO: support for advanced filtering (price ranges, etc.) could be desirable
        pub fn get_auctions(&self) -> BTreeMap<u64, Vec<ComponentAddress>> {
            self.auctions.clone()
        }

        // returns a badge used to cancel the sell order in the future
        // the badge will contain immutable metadata referencing the nft being sold
        pub fn create_auction(
            &mut self,
            nft_bucket: Bucket,
            seller_address: ComponentAddress,
            min_price: Option<Amount>,
            buy_price: Option<Amount>,
            epoch_period: u64,
        ) -> (ComponentAddress, Bucket) {
            // init the auction component
            let (auction_component, seller_badge): (ComponentAddress, Bucket) = TemplateManager::get(self.auction_template)
                .call("new".to_string(), call_args![
                    nft_bucket,
                    seller_address,
                    min_price,
                    buy_price,
                    epoch_period
                ]);

            // add the new auction component to the index
            let ending_epoch = Consensus::current_epoch() + epoch_period;
            if let Some(auctions) = self.auctions.get_mut(&ending_epoch) {
                auctions.push(auction_component);
            } else {
                self.auctions.insert(ending_epoch, vec![auction_component]);
            }
            
            (auction_component, seller_badge)
        }
    }
}
