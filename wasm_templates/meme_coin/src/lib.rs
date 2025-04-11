use tari_template_lib::prelude::*;

#[template]
pub mod {{ project-name | snake_case }} {
    use super::*;

    pub struct {{ project-name | upper_camel_case }} {
        token_vault: Vault,
    }

    impl {{ project-name | upper_camel_case }} {
        pub fn create(
            initial_supply: Amount,
            token_symbol: String,
            token_image_url: Option<String>,
            metadata: Metadata,
        ) -> Component<Self> {
            // initial coin with supply
            let mut coins = ResourceBuilder::fungible()
                .with_token_symbol(&token_symbol)
                .with_metadata(metadata)
                .with_owner_rule(OwnerRule::OwnedBySigner);
            if let Some(url) = token_image_url {
                coins = coins.with_image_url(url);
            }
            let coins = coins.initial_supply(initial_supply);

            Component::new(Self {
                token_vault: Vault::from_bucket(coins),
            })
                .with_owner_rule(OwnerRule::OwnedBySigner)
                .create()
        }

        pub fn set_access_rules(&mut self, access_rules: ResourceAccessRules) {
            ResourceManager::get(self.token_vault.resource_address()).set_access_rules(access_rules)
        }

        pub fn burn(&mut self, amount: Amount) {
            let bucket = self.token_vault.withdraw(amount);
            bucket.burn();
        }

        pub fn withdraw(&mut self, amount: Amount) -> Bucket {
            self.token_vault.withdraw(amount)
        }

        pub fn mint(&mut self, amount: Amount) {
            let bucket =
                ResourceManager::get(self.token_vault.resource_address()).mint_fungible(amount);
            self.token_vault.deposit(bucket);
        }

        pub fn vault_address(&self) -> ResourceAddress {
            self.token_vault.resource_address()
        }

        pub fn total_supply(&self) -> Amount {
            ResourceManager::get(self.token_vault.resource_address()).total_supply()
        }

        pub fn balance(&self) -> Amount {
            self.token_vault.balance()
        }
    }
}
