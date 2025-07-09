use tari_template_lib::prelude::*;

#[template]
pub mod {{ project-name | snake_case }} {
    use super::*;

    pub struct {{ project-name | upper_camel_case }} {
        vault: Vault,
    }

    impl {{ project-name | upper_camel_case }} {
        pub fn mint(initial_supply: Amount, token_symbol: String) -> Component<Self> {
            let coins = ResourceBuilder::fungible()
                .with_token_symbol(&token_symbol)
                .initial_supply(initial_supply);

            Component::new(Self { vault: Vault::from_bucket(coins) })
                .with_access_rules(AccessRules::allow_all())
                .create()
        }

        pub fn resource_address(&self) -> ResourceAddress {
            self.vault.resource_address()
        }

        pub fn take_free_coins(&mut self, amount: Amount) -> Bucket {
            self.vault.withdraw(amount)
        }

        pub fn balance(&self) -> Amount {
            self.vault.balance()
        }

        // TODO: we can make a fungible utility template with these common operations
        pub fn burn_coins(&mut self, amount: Amount) {
            let bucket = self.vault.withdraw(amount);
            bucket.burn();
        }

        pub fn total_supply(&self) -> Amount {
            ResourceManager::get(self.vault.resource_address()).total_supply()
        }
    }
}

