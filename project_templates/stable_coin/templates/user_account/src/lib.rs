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

use tari_template_lib::prelude::*;

#[template]
mod stable_coin {
    use super::*;
    use stable_coin_common::CreateNewUserAccountResponse;
    use stable_coin_common::IssuerApi;

    pub struct PrivateStableCoinUserAccount {
        token_vault: Vault,
        user_badge: Vault,
        is_frozen: bool,
        issuer: IssuerApi,
    }

    impl PrivateStableCoinUserAccount {
        /// Creates a new stable coin user account
        pub fn create(
            issuer_component: ComponentAddress,
            user_public_key: RistrettoPublicKeyBytes,
            admin_proof: Proof,
        ) -> Component<Self> {
            let issuer = IssuerApi::new(issuer_component);
            // TODO: implement address allocations so that we can register the address before creating the component

            let CreateNewUserAccountResponse {
                token_resource,
                user_badge,
                admin_only_access_rule,
            } = issuer.create_user_account(admin_proof, user_public_key);

            // Create component access rules
            let require_user_permission = AccessRule::Restricted(Require(RequireRule::Require(
                user_badge.resource_address().into(),
            )));
            let component_access_rules = AccessRules::new()
                .add_method_rule("transfer_to", AccessRule::AllowAll)
                .add_method_rule("deposit", require_user_permission)
                .add_method_rule("deposit_auth_badge", admin_only_access_rule.clone())
                .add_method_rule("freeze_account", admin_only_access_rule.clone())
                .add_method_rule("unfreeze_account", admin_only_access_rule)
                // Deny to all but the owner
                .default(AccessRule::DenyAll);

            // Create component
            Component::new(Self {
                token_vault: Vault::new_empty(token_resource),
                user_badge: Vault::from_bucket(user_badge),
                is_frozen: false,
                issuer,
            })
                .with_access_rules(component_access_rules)
                .with_owner_rule(OwnerRule::OwnedBySigner)
                .create()
        }

        pub fn transfer_to(
            &mut self,
            destination_account: ComponentAddress,
            withdraw_proof: ConfidentialWithdrawProof,
        ) {
            if self.is_frozen {
                panic!("Account is frozen");
            }
            let proof = self.user_badge.create_proof();
            self.issuer
                .check_transfer(proof.clone(), destination_account);

            let funds =
                proof.authorize_with(|| self.token_vault.withdraw_confidential(withdraw_proof));

            ComponentManager::get(destination_account)
                .call::<_, ()>("deposit", args![proof, funds]);
            proof.drop();

            emit_event(
                "transfer_to",
                [("destination", destination_account.to_string())],
            );
        }

        pub fn deposit(&mut self, proof: Proof, funds: Bucket) {
            if self.is_frozen {
                panic!("Account is frozen");
            }
            let _auth = proof.authorize();
            let amount = funds.amount();
            self.token_vault.deposit(funds);
            emit_event("deposit", [("amount", amount.to_string())]);
        }

        pub fn deposit_auth_badge(&mut self, admin_proof: Proof, badge: Bucket) {
            let _auth = admin_proof.authorize();
            assert_eq!(
                self.user_badge.balance(),
                Amount(0),
                "User already has a badge"
            );
            assert_eq!(
                badge.amount(),
                Amount(1),
                "Cannot give the user more than one badge"
            );
            self.user_badge.deposit(badge);
            emit_event("deposit_auth_badge", [] as [(&str, String); 0])
        }

        pub fn freeze_account(&mut self, _admin_proof: Proof) {
            self.is_frozen = true;
            emit_event("freeze_account", [] as [(&str, String); 0]);
        }

        pub fn unfreeze_account(&mut self, _admin_proof: Proof) {
            self.is_frozen = false;
            emit_event("unfreeze_account", [] as [(&str, String); 0]);
        }
    }
}
