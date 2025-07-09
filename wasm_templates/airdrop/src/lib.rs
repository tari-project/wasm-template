use std::collections::BTreeSet;
use tari_template_lib::prelude::*;

#[template]
pub mod {{ project-name | snake_case }} {
    use super::*;

    pub struct {{ project-name | upper_camel_case }} {
        allow_list: BTreeSet<ComponentAddress>,
        is_airdrop_open: bool,
        claimed_count: u128,
        vault: Vault,
    }

    impl {{ project-name | upper_camel_case }} {
        pub fn new() -> Component<Self> {
            let bucket = ResourceBuilder::non_fungible()
                .with_token_symbol("{{ project-name | shouty_kebab_case }}")
                .with_owner_rule(OwnerRule::OwnedBySigner)
                .initial_supply((1..=100).map(NonFungibleId::from_u32));

            Component::new(Self {
                allow_list: BTreeSet::new(),
                is_airdrop_open: false,
                claimed_count: 0,
                vault: Vault::from_bucket(bucket),
            })
                .with_access_rules(AccessRules::allow_all())
                .create()
        }

        pub fn add_recipient(&mut self, address: ComponentAddress) {
            assert!(self.is_airdrop_open, "Airdrop already started");
            assert!(self.allow_list.len() < 100, "Airdrop allow list is full");
            assert!(
                !self.allow_list.contains(&address),
                "Address already in allow list"
            );
            self.allow_list.insert(address);
        }

        pub fn open_airdrop(&mut self) {
            assert!(!self.is_airdrop_open, "Airdrop already open");
            self.is_airdrop_open = true;
        }

        pub fn claim_any(&mut self, address: ComponentAddress) -> Bucket {
            assert!(self.is_airdrop_open, "Airdrop is not open");
            // Note: this does not enforce that the token is deposited in an address from the allow list
            assert!(
                self.allow_list.remove(&address),
                "Address {} is not in allow list or has already been claimed",
                address
            );

            self.claimed_count += 1;
            self.vault.withdraw(1)
        }

        pub fn claim_specific(&mut self, address: ComponentAddress, id: NonFungibleId) -> Bucket {
            assert!(self.is_airdrop_open, "Airdrop is not open");
            assert!(
                self.allow_list.remove(&address),
                "Address {} is not in allow list or has already been claimed",
                address
            );

            self.claimed_count += 1;
            self.vault.withdraw_non_fungibles(Some(id))
        }

        pub fn total_supply(&self) -> Amount {
            ResourceManager::get(self.vault.resource_address()).total_supply()
        }

        pub fn num_claimed(&self) -> u128 {
            self.claimed_count
        }

        pub fn vault_balance(&self) -> Amount {
            self.vault.balance()
        }

        pub fn set_access_rules(&mut self, access_rules: ResourceAccessRules) {
            ResourceManager::get(self.vault.resource_address()).set_access_rules(access_rules)
        }
    }
}
