//   Copyright 2022. The Tari Project
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

#[template]
mod nft_template {
    use super::*;

    pub struct Nft {
        address: ResourceAddress,
    }

    impl Nft {
        pub fn new() -> Self {
            // Create the non-fungible resource
            let address = ResourceBuilder::non_fungible().build();
            Self {
                address,
            }
        }

        pub fn get_resource_address(&self) -> ResourceAddress {
            self.address
        }

        pub fn mint(&mut self) -> Bucket {
            // Mint a new token with a random ID
            let id = NonFungibleId::random();
            // These are characteristic of the NFT and are immutable
            let mut immutable_data = Metadata::new();
            immutable_data
                .insert("name", format!("NFT-{}", id))
                .insert("image_url", format!("https://nft.storage/nft-{}.png", id));
            // TODO: Custom data that is mutable by the token owner
            ResourceManager::get(self.address).mint_non_fungible(id, NftToken::new(immutable_data, Vec::new()))
        }

        pub fn total_supply(&self) -> Amount {
            ResourceManager::get(self.address).total_supply()
        }
    }
}
