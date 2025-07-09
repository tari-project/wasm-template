use tari_template_lib::prelude::*;

#[template]
mod {{ project-name | snake_case }}_ico {
    use super::*;

    pub struct {{ project-name | upper_camel_case }}Ico {
        ico_tokens: Vault,
        reward_coins: Vault,
        token_price: Amount,
    }

    impl {{ project-name | upper_camel_case }}Ico {
        pub fn new(
            symbol: String,
            initial_supply: Amount,
            price: Amount,
        ) -> (Component<Self>, Bucket) {
            let coins = ResourceBuilder::fungible()
                .with_token_symbol(symbol)
                .with_owner_rule(OwnerRule::OwnedBySigner)
                .initial_supply(initial_supply);

            let admin_badge = ResourceBuilder::non_fungible()
                .with_owner_rule(OwnerRule::OwnedBySigner)
                .initial_supply(vec![NonFungibleId::random()]);

            let comp_access_rules = ComponentAccessRules::new()
                .default(AccessRule::AllowAll)
                .add_method_rule("withdraw", rule!(resource(admin_badge.resource_address())));

            (
                Component::new(Self {
                    ico_tokens: Vault::from_bucket(coins),
                    reward_coins: Vault::new_empty(XTR),
                    token_price: price,
                })
                    .with_owner_rule(OwnerRule::OwnedBySigner)
                    .with_access_rules(comp_access_rules)
                    .create(),
                admin_badge
            )
        }

        pub fn buy(&mut self, xtr_coins: Bucket) -> Bucket {
            assert_eq!(xtr_coins.resource_address(), XTR, "You must pay with XTR!");
            if xtr_coins.amount() < self.token_price {
                panic!("Insufficient funds! You need more XTR to buy ICOs.");
            }
            let ico_tokens_count = xtr_coins.amount() / self.token_price;
            self.reward_coins.deposit(xtr_coins);
            self.ico_tokens.withdraw(ico_tokens_count)
        }

        pub fn xtr_balance(&self) -> Amount {
            self.reward_coins.balance()
        }

        pub fn ico_resource_address(&self) -> ResourceAddress {
            self.ico_tokens.resource_address()
        }

        pub fn withdraw(&mut self, amount: Amount) -> Bucket {
            self.reward_coins.withdraw(amount)
        }
    }
}
