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
                .call("new".to_string(), args![
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
