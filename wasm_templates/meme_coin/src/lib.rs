use tari_template_lib::prelude::*;

#[template]
pub mod {{ project-name | snake_case }} {
    use super::*;

    pub struct {{ project-name | upper_camel_case }} {
        token_vault: Vault,
        admin_auth_resource: ResourceAddress,
    }

    impl {{ project-name | upper_camel_case }} {
        pub fn mint(
            initial_supply: Amount,
            token_symbol: String,
            token_image_url: Option<String>,
        ) -> (Component<Self>, Bucket) {
            // initial coin with supply
            let mut coins = ResourceBuilder::fungible().with_token_symbol(&token_symbol);
            if let Some(url) = token_image_url {
                coins = coins.with_image_url(url);
            }
            let coins = coins.initial_supply(initial_supply);

            let admin_badge =
                ResourceBuilder::non_fungible().initial_supply(vec![NonFungibleId::random()]);

            // Create admin access rule
            let admin_resource = admin_badge.resource_address();
            let require_admin =
                AccessRule::Restricted(Require(RequireRule::Require(admin_resource.into())));

            (
                Component::new(Self {
                    token_vault: Vault::from_bucket(coins),
                    admin_auth_resource: admin_resource,
                })
                .with_access_rules(
                    AccessRules::new()
                        .add_method_rule("total_supply", AccessRule::AllowAll)
                        .default(require_admin.clone()),
                )
                .with_owner_rule(OwnerRule::OwnedBySigner)
                .create(),
                admin_badge,
            )
        }

        pub fn burn(&mut self, amount: Amount) {
            let bucket = self.token_vault.withdraw(amount);
            bucket.burn();
        }

        pub fn withdraw(&mut self, amount: Amount) -> Bucket {
            self.token_vault.withdraw(amount)
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

