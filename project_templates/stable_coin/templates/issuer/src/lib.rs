//   Copyright 2023. The Tari Project
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

mod deny_list;
use tari_template_lib::prelude::*;

#[template]
mod stable_coin {
    use super::*;

    use deny_list::*;
    use stable_coin_common::CreateNewUserAccountResponse;
    use std::collections::HashSet;

    pub struct PrivateStableCoinIssuer {
        token_vault: Vault,
        admin_auth_resource: ResourceAddress,
        user_auth_resource: ResourceAddress,

        user_account_template: TemplateAddress,
        users: HashSet<RistrettoPublicKeyBytes>,
        deny_list: DenyList,
        denied_user_badges: Vault,
    }

    impl PrivateStableCoinIssuer {
        /// Instantiates a new stable coin component, returning the component and an bucket containing an admin badge
        pub fn instantiate(
            initial_token_supply: ConfidentialOutputStatement,
            token_symbol: String,
            user_account_template: TemplateAddress,
            token_metadata: Metadata,
        ) -> (Component<Self>, Bucket) {
            let provider_name = token_metadata
                .get("provider_name")
                .expect("provider_name metadata entry is required");

            // Create admin badge resource
            let admin_badge = ResourceBuilder::non_fungible()
                .initial_supply(vec![NonFungibleId::random()]);

            // Create admin access rules
            let admin_resource = admin_badge.resource_address();
            let require_admin = AccessRule::Restricted(RestrictedAccessRule::Require(
                RequireRule::Require(admin_resource.into()),
            ));

            // Create user badge resource
            let user_auth_resource = ResourceBuilder::non_fungible()
                .add_metadata("provider_name", provider_name)
                .mintable(require_admin.clone())
                .depositable(require_admin.clone())
                .recallable(require_admin.clone())
                .update_non_fungible_data(require_admin.clone())
                .build();

            // Create user access rules
            let require_user =
                AccessRule::Restricted(RestrictedAccessRule::Require(RequireRule::AnyOf(vec![
                    admin_resource.into(),
                    user_auth_resource.into(),
                ])));

            // Create tokens resource with initial supply
            let initial_tokens = ResourceBuilder::confidential()
                .with_token_symbol(token_symbol)
                .with_metadata(token_metadata)
                // Access rules
                .mintable(require_admin.clone())
                .burnable(require_admin.clone())
                .depositable(require_user.clone())
                .withdrawable(require_user.clone())
                .initial_supply(initial_token_supply);

            // Create component access rules
            let component_access_rules = AccessRules::new()
                .add_method_rule("total_supply", AccessRule::AllowAll)
                .add_method_rule("check_transfer", AccessRule::AllowAll)
                .default(require_admin);

            // Create component
            let component = Component::new(Self {
                token_vault: Vault::from_bucket(initial_tokens),
                user_auth_resource,
                user_account_template,
                admin_auth_resource: admin_badge.resource_address(),
                deny_list: DenyList::new(),
                users: HashSet::new(),
                denied_user_badges: Vault::new_empty(user_auth_resource),
            })
            .with_access_rules(component_access_rules)
            // Access is entirely controlled by anyone with an admin badge
            .with_owner_rule(OwnerRule::None)
            .create();

            (component, admin_badge)
        }

        pub fn create_new_admin(&mut self) -> Bucket {
            let badge = ResourceManager::get(self.admin_auth_resource).mint_non_fungible(
                NonFungibleId::random(),
                &(),
                &(),
            );
            emit_event("create_new_admin", [] as [(&str, String); 0]);
            badge
        }

        pub fn delete_admin(&mut self, vault_id: VaultId, id: NonFungibleId) {
            let badge =
                ResourceManager::get(self.admin_auth_resource).recall_non_fungible(vault_id, id);
            badge.burn();
            emit_event("delete_admin", [] as [(&str, String); 0]);
        }

        /// Increase token supply by amount.
        pub fn increase_supply(&mut self, proof: ConfidentialOutputStatement) {
            let new_tokens =
                ResourceManager::get(self.token_vault.resource_address()).mint_confidential(proof);
            self.token_vault.deposit(new_tokens);
            emit_event("increase_supply", [] as [(&str, String); 0]);
        }

        /// Decrease token supply by amount.
        pub fn decrease_supply(&mut self, burn_proof: ConfidentialWithdrawProof) {
            let tokens = self.token_vault.withdraw_confidential(burn_proof);
            tokens.burn();
            emit_event("decrease_supply", [] as [(&str, String); 0]);
        }

        pub fn total_supply(&self) -> Amount {
            ResourceManager::get(self.token_vault.resource_address()).total_supply()
        }

        pub fn withdraw_confidential(
            &mut self,
            withdraw: ConfidentialWithdrawProof,
            description: String,
        ) -> Bucket {
            let bucket = self.token_vault.withdraw_confidential(withdraw);
            emit_event("withdraw_confidential", [("description", description)]);
            bucket
        }

        pub fn withdraw_revealed(&mut self, amount: Amount, description: String) -> Bucket {
            let bucket = self.token_vault.withdraw(amount);
            emit_event(
                "withdraw_revealed",
                [("amount", amount.to_string()), ("description", description)],
            );
            bucket
        }

        pub fn deposit(&mut self, bucket: Bucket) {
            let amount = bucket.amount();
            self.token_vault.deposit(bucket);
            emit_event("deposit", [("amount", amount.to_string())]);
        }

        pub fn add_user_to_deny_list(
            &mut self,
            admin_proof: Proof,
            user_public_key: RistrettoPublicKeyBytes,
            component_address: ComponentAddress,
            vault_id: VaultId,
        ) {
            // TODO: we should fetch the associated component address for the user, however this requires that we know the user
            if !self
                .deny_list
                .insert_entry(user_public_key, component_address)
            {
                panic!("User already on deny list");
            }

            admin_proof.authorize_with(|| {
                // TODO: the authorization doesnt carry though to the cross-component call so this doesnt actually do anything.
                //       Doing this seems risky so will have to be evaluated, for now we bring the proof into scope by passing it in as an argument.
                ComponentManager::get(component_address)
                    .call::<_, ()>("freeze_account", args![admin_proof]);

                let recalled = ResourceManager::get(self.user_auth_resource).recall_non_fungible(
                    vault_id,
                    NonFungibleId::from_u256(user_public_key.into_array()),
                );
                self.denied_user_badges.deposit(recalled);
            });

            emit_event(
                "add_user_to_deny_list",
                [("user_public_key", user_public_key.to_string())],
            );
        }

        pub fn remove_user_from_deny_list(
            &mut self,
            admin_proof: Proof,
            user_public_key: RistrettoPublicKeyBytes,
        ) {
            let Some(component) = self.deny_list.remove_by_public_key(&user_public_key) else {
                panic!("User not found in blacklist");
            };

            admin_proof.authorize_with(|| {
                // TODO: the authorization doesnt carry though to the cross-component call so this doesnt actually do anything.
                //       Doing this seems risky so will have to be evaluated, for now we bring the proof into scope by passing it in as an argument.
                ComponentManager::get(component)
                    .call::<_, ()>("unfreeze_account", args![admin_proof]);

                let user_badge = self
                    .denied_user_badges
                    .withdraw_non_fungible(NonFungibleId::from_u256(user_public_key.into_array()));

                ComponentManager::get(component)
                    .call::<_, ()>("deposit_auth_badge", args![admin_proof, user_badge]);
            });
            emit_event(
                "remove_user_from_deny_list",
                [("user_public_key", user_public_key.to_string())],
            );
        }

        pub fn create_user_account(
            &mut self,
            admin_proof: Proof,
            user_public_key: RistrettoPublicKeyBytes,
        ) -> CreateNewUserAccountResponse {
            if self.user_exists(&user_public_key) {
                panic!("User already exists");
            }
            if self.is_user_denied(&user_public_key) {
                panic!("User is on deny list");
            }

            let _auth = admin_proof.authorize();

            let user_badge = ResourceManager::get(self.user_auth_resource).mint_non_fungible(
                NonFungibleId::from_u256(user_public_key.into_array()),
                &(),
                &(),
            );

            self.users.insert(user_public_key);

            let admin_only = AccessRule::Restricted(Require(RequireRule::Require(
                self.admin_auth_resource.into(),
            )));
            CreateNewUserAccountResponse {
                token_resource: self.token_vault.resource_address(),
                user_badge,
                admin_only_access_rule: admin_only,
            }
        }

        pub fn check_transfer(&self, proof: Proof, destination_account: ComponentAddress) {
            proof.assert_resource(self.user_auth_resource);
            let template_address =
                ComponentManager::get(destination_account).get_template_address();
            assert_eq!(
                template_address, self.user_account_template,
                "Not a user account template"
            );

            if self.deny_list.contains_component(&destination_account) {
                panic!("Transfer denied to account {}", destination_account)
            }
        }

        fn user_exists(&self, user_public_key: &RistrettoPublicKeyBytes) -> bool {
            self.users.contains(user_public_key)
        }

        fn is_user_denied(&self, user_public_key: &RistrettoPublicKeyBytes) -> bool {
            self.deny_list.contains_public_key(user_public_key)
        }
    }
}
