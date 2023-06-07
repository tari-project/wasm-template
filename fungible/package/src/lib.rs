use tari_template_lib::prelude::*;

#[template]
mod faucet_template {
    use super::*;

    pub struct TestFaucet {
        vault: Vault,
    }

    impl TestFaucet {
        pub fn mint(initial_supply: Amount, token_symbol: String) -> Self {
            let coins = ResourceBuilder::fungible(&token_symbol)
                .initial_supply(initial_supply)
                .build_bucket();

            Self {
                vault: Vault::from_bucket(coins),
            }
        }

        pub fn take_free_coins(&mut self) -> Bucket {
            debug("Withdrawing 1000 coins from faucet");
            self.vault.withdraw(Amount::new(1000))
        }

        // TODO: we can make a fungible utility template with these common operations
        pub fn burn_coins(&mut self, amount: Amount) {
            let mut bucket = self.vault.withdraw(amount);
            bucket.burn();
        }

        pub fn total_supply(&self) -> Amount {
            ResourceManager::get(self.vault.resource_address()).total_supply()
        }
    }
}
