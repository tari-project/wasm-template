mod user_data;
mod wrapped_exchange_token;

use user_data::{UserData, UserId, UserMutableData};

use tari_template_lib::prelude::*;

#[template]
mod template {
    use super::*;
    use crate::wrapped_exchange_token::{ExchangeFee, WrappedExchangeToken};
    use std::collections::BTreeSet;
    use tari_template_lib::engine;

    const DEFAULT_WRAPPED_TOKEN_EXCHANGE_FEE: ExchangeFee = ExchangeFee::Fixed(amount!(5));

    pub struct {{ project-name | upper_camel_case }} {
        token_vault: Vault,
        user_auth_resource: ResourceAddress,
        admin_auth_resource: ResourceAddress,
        blacklisted_users: Vault,
        wrapped_token: Option<WrappedExchangeToken>,
        total_supply: Amount,
    }

    impl {{ project-name | upper_camel_case }} {
        /// Instantiates a new stable coin component, returning a bucket containing an admin badge
        pub fn instantiate(
            initial_token_supply: Amount,
            token_symbol: String,
            token_metadata: Metadata,
            view_key: RistrettoPublicKeyBytes,
            enable_wrapped_token: bool,
        ) -> Bucket {
            let provider_name = token_metadata
                .get("provider_name")
                .filter(|v| !v.trim().is_empty())
                .expect("provider_name metadata entry is required");

            // Create admin badge resource
            let admin_badge =
                ResourceBuilder::non_fungible().initial_supply(Some(NonFungibleId::from_u64(0)));

            // Create admin access rules
            let admin_resource = admin_badge.resource_address();
            let require_admin =
                AccessRule::Restricted(Require(RequireRule::Require(admin_resource.into())));

            // Create user badge resource
            let user_auth_resource = ResourceBuilder::non_fungible()
                .add_metadata("provider_name", provider_name.trim())
                .depositable(require_admin.clone())
                .recallable(require_admin.clone())
                .update_non_fungible_data(require_admin.clone())
                .build();

            // Create user access rules
            let require_user = rule!(any_of(
                resource(admin_resource),
                resource(user_auth_resource)
            ));

            let component_alloc = CallerContext::allocate_component_address(None);
            // Create tokens resource with initial supply
            let initial_supply_proof =
                ConfidentialOutputStatement::mint_revealed(initial_token_supply);
            let initial_tokens = ResourceBuilder::confidential()
                .with_metadata(token_metadata.clone())
                .with_token_symbol(&token_symbol)
                // Access rules
                .mintable(require_admin.clone())
                .burnable(require_admin.clone())
                .depositable(require_user.clone())
                .withdrawable(require_user.clone())
                .recallable(require_admin.clone())
                .with_authorization_hook(component_alloc.get_address(), "authorize_user_deposit")
                .with_view_key(view_key)
                .initial_supply(initial_supply_proof);

            // Create tokens resource with initial supply
            let wrapped_token = if enable_wrapped_token {
                let wrapped_resource = ResourceBuilder::fungible()
                    .with_metadata(token_metadata)
                    .with_token_symbol(format!("w{token_symbol}"))
                    // Access rules
                    .mintable(require_admin.clone())
                    .burnable(require_admin.clone())
                    .initial_supply(initial_token_supply);

                Some(WrappedExchangeToken {
                    vault: Vault::from_bucket(wrapped_resource),
                    exchange_fee: DEFAULT_WRAPPED_TOKEN_EXCHANGE_FEE,
                })
            } else {
                None
            };

            // Create component access rules
            let component_access_rules = AccessRules::new()
                .add_method_rule("total_supply", AccessRule::AllowAll)
                .add_method_rule("exchange_stable_for_wrapped_tokens", require_user.clone())
                .add_method_rule("exchange_wrapped_for_stable_tokens", require_user.clone())
                .add_method_rule("authorize_user_deposit", AccessRule::AllowAll)
                .default(require_admin);

            // Create component
            let _component = Component::new(Self {
                token_vault: Vault::from_bucket(initial_tokens),
                user_auth_resource,
                admin_auth_resource: admin_badge.resource_address(),
                blacklisted_users: Vault::new_empty(user_auth_resource),
                wrapped_token,
                total_supply: initial_token_supply,
            })
            .with_address_allocation(component_alloc)
            .with_access_rules(component_access_rules)
            // Access is controlled by anyone with an admin badge, there is no single owner
            .with_owner_rule(OwnerRule::None)
            .create();

            admin_badge
        }

        pub fn authorize_user_deposit(&self, action: ResourceAuthAction, caller: AuthHookCaller) {
            match action {
                ResourceAuthAction::Deposit => {
                    let Some(component_state) = caller.component_state() else {
                        panic!("deposit not permitted from static template function")
                    };
                    info!(
                        "Authorizing deposit for user with component {}",
                        caller.component().unwrap()
                    );
                    let user_account =
                        Account::from_value(component_state).expect("not called from an account");
                    let vault = user_account
                        .get_vault_by_resource(&self.user_auth_resource)
                        .expect("This account does not have permission to deposit");

                    // User must own a badge of this user auth resource. The badge may be locked when sending to self.
                    if vault.balance().is_zero() && vault.locked_balance().is_zero() {
                        panic!("This account does not have permission to deposit");
                    }
                }
                _ => {
                    // Withdraws etc are permitted as per normal resource access rules
                }
            }
        }

        /// Increase token supply by amount.
        pub fn increase_supply(&mut self, amount: Amount) {
            let proof = ConfidentialOutputStatement::mint_revealed(amount);
            let new_tokens = self.token_vault_manager().mint_confidential(proof);
            self.token_vault.deposit(new_tokens);
            self.total_supply += amount;

            if let Some(ref mut wrapped_token) = self.wrapped_token {
                let new_tokens =
                    ResourceManager::get(wrapped_token.resource_address()).mint_fungible(amount);
                wrapped_token.vault_mut().deposit(new_tokens);
            }

            emit_event("increase_supply", [("amount", amount.to_string())]);
        }

        /// Decrease token supply by amount.
        pub fn decrease_supply(&mut self, amount: Amount) {
            let proof = ConfidentialWithdrawProof::revealed_withdraw(amount);

            let tokens = self.token_vault.withdraw_confidential(proof);
            tokens.burn();
            self.total_supply -= amount;

            if let Some(ref mut wrapped_token) = self.wrapped_token {
                let wrapped_tokens = wrapped_token.vault_mut().withdraw(amount);
                wrapped_tokens.burn();
            }

            emit_event(
                "decrease_supply",
                [("revealed_burn_amount", amount.to_string())],
            );
        }

        pub fn total_supply(&self) -> Amount {
            self.total_supply
        }

        pub fn withdraw(&mut self, amount: Amount) -> Bucket {
            let proof = ConfidentialWithdrawProof::revealed_withdraw(amount);
            let bucket = self.token_vault.withdraw_confidential(proof);
            emit_event(
                "withdraw",
                [("amount_withdrawn", bucket.amount().to_string())],
            );
            bucket
        }

        pub fn deposit(&mut self, bucket: Bucket) {
            let amount = bucket.amount();
            self.token_vault.deposit(bucket);
            emit_event("deposit", [("amount", amount.to_string())]);
        }

        /// Allow the user to exchange their tokens for wrapped tokens
        pub fn exchange_stable_for_wrapped_tokens(
            &mut self,
            proof: Proof,
            confidential_bucket: Bucket,
        ) -> Bucket {
            assert_eq!(
                confidential_bucket.resource_address(),
                self.token_vault.resource_address(),
                "The bucket must contain the same resource as the token vault"
            );

            // Check the bucket does not contain any non-revealed confidential tokens
            assert_eq!(
                confidential_bucket.count_confidential_commitments(),
                0,
                "No confidential outputs allowed when exchanging for wrapped tokens"
            );

            assert!(
                !confidential_bucket.amount().is_zero(),
                "The bucket must contain some tokens"
            );

            proof.assert_resource(self.user_auth_resource);
            let badges = proof.get_non_fungibles();
            assert_eq!(badges.len(), 1, "The proof must contain exactly one badge");
            let badge = badges.into_iter().next().unwrap();
            let badge = self.user_badge_manager().get_non_fungible(&badge);
            let user = badge.get_data::<UserData>();
            let user_data = badge.get_mutable_data::<UserMutableData>();

            let amount = confidential_bucket.amount();
            assert!(
                amount <= user_data.wrapped_exchange_limit,
                "Exchange limit exceeded"
            );

            self.set_user_wrapped_exchange_limit(
                user.user_id,
                user_data.wrapped_exchange_limit - amount,
            );

            let fee = self
                .wrapped_token_mut()
                .exchange_fee()
                .calculate_fee(amount);
            let new_amount = amount
                .checked_sub(fee)
                .expect("Insufficient funds to pay exchange fee");

            self.token_vault.deposit(confidential_bucket);

            let wrapped_tokens = self.wrapped_token_mut().vault_mut().withdraw(new_amount);

            emit_event(
                "exchange_stable_for_wrapped_tokens",
                [
                    ("user_id", user.user_id.to_string()),
                    ("amount", amount.to_string()),
                    ("fee", fee.to_string()),
                ],
            );

            wrapped_tokens
        }

        /// Allow the user to exchange their wrapped tokens for stable coin tokens
        pub fn exchange_wrapped_for_stable_tokens(
            &mut self,
            proof: Proof,
            wrapped_bucket: Bucket,
        ) -> Bucket {
            proof.assert_resource(self.user_auth_resource);

            assert_eq!(
                wrapped_bucket.resource_address(),
                self.wrapped_token_mut().vault().resource_address(),
                "The bucket must contain the same resource as the wrapped token vault"
            );

            assert!(
                !wrapped_bucket.amount().is_zero(),
                "The bucket must contain some tokens"
            );

            let badges = proof.get_non_fungibles();
            assert_eq!(badges.len(), 1, "The proof must contain exactly one badge");
            let badge = badges.into_iter().next().unwrap();
            let badge = self.user_badge_manager().get_non_fungible(&badge);
            let user = badge.get_data::<UserData>();

            let amount = wrapped_bucket.amount();

            self.wrapped_token_mut().vault_mut().deposit(wrapped_bucket);

            // TODO: we should be able to call withdraw on the confidential resource without creating a revealed proof
            let withdraw = ConfidentialWithdrawProof::revealed_withdraw(amount);
            let tokens = self.token_vault.withdraw_confidential(withdraw);

            emit_event(
                "exchange_wrapped_for_stable_tokens",
                [
                    ("user_id", user.user_id.to_string()),
                    ("amount", amount.to_string()),
                    ("fee", 0.to_string()),
                ],
            );

            tokens
        }

        pub fn recall_tokens(
            &mut self,
            user_id: UserId,
            commitments: BTreeSet<PedersenCommitmentBytes>,
            amount: Amount,
        ) {
            // Fetch the user badge
            let badge = self.user_badge_manager().get_non_fungible(&user_id.into());
            let user = badge.get_data::<UserData>();

            let component_manager = engine().component_manager(user.user_account);
            let account = component_manager.get_state::<Account>();

            let vault = account
                .get_vault_by_resource(&self.token_vault.resource_address())
                .expect("User account does not have a stable coin vault");
            let vault_id = vault.vault_id();
            let num_commitments = commitments.len();

            let bucket =
                self.token_vault_manager()
                    .recall_confidential(vault_id, commitments, amount);
            self.token_vault.deposit(bucket);

            emit_event(
                "recall_tokens",
                [
                    ("user_id", user_id.to_string()),
                    ("revealed_amount", amount.to_string()),
                    ("num_commitments", num_commitments.to_string()),
                ],
            );
        }

        pub fn create_new_admin(&mut self, employee_id: String) -> Bucket {
            let id = NonFungibleId::random();
            emit_event("create_new_admin", [("admin_id", id.to_string())]);
            let mut metadata = Metadata::new();
            metadata.insert("employee_id", employee_id);
            let badge = ResourceManager::get(self.admin_auth_resource).mint_non_fungible(
                id,
                &metadata,
                &(),
            );
            badge
        }

        pub fn create_new_user(
            &mut self,
            user_id: UserId,
            user_account: ComponentAddress,
        ) -> Bucket {
            // TODO: configurable?
            const DEFAULT_EXCHANGE_LIMIT: Amount = amount!(1000);

            let badge = self.user_badge_manager().mint_non_fungible(
                user_id.into(),
                &UserData {
                    user_id,
                    user_account,
                    // TODO: real time not implemented
                    created_at: 0,
                },
                &UserMutableData {
                    is_blacklisted: false,
                    wrapped_exchange_limit: DEFAULT_EXCHANGE_LIMIT,
                },
            );
            emit_event("create_new_user", [("user_id", user_id.to_string())]);
            badge
        }

        pub fn set_user_exchange_limit(&mut self, user_id: UserId, limit: Amount) {
            assert!(limit.is_positive(), "Exchange limit must be positive");
            let non_fungible_id: NonFungibleId = user_id.into();

            let manager = self.user_badge_manager();
            let user_badge = manager.get_non_fungible(&non_fungible_id);
            let user_data = user_badge.get_mutable_data::<UserMutableData>();
            manager.update_non_fungible_data(
                non_fungible_id,
                &UserMutableData {
                    wrapped_exchange_limit: limit,
                    ..user_data
                },
            );

            let admin = CallerContext::transaction_signer_public_key();
            emit_event(
                "set_user_exchange_limit",
                [
                    ("user_id", user_id.to_string()),
                    ("limit", limit.to_string()),
                    ("admin", admin.to_string()),
                ],
            );
        }

        pub fn blacklist_user(&mut self, vault_id: VaultId, user_id: UserId) {
            let non_fungible_id: NonFungibleId = user_id.into();

            let manager = self.user_badge_manager();
            let recalled = manager.recall_non_fungible(vault_id, non_fungible_id.clone());
            let user_badge = manager.get_non_fungible(&non_fungible_id);
            let user_data = user_badge.get_mutable_data::<UserMutableData>();
            manager.update_non_fungible_data(
                non_fungible_id,
                &UserMutableData {
                    is_blacklisted: true,
                    ..user_data
                },
            );

            self.blacklisted_users.deposit(recalled);
            emit_event("blacklist_user", [("user_id", user_id.to_string())]);
        }

        pub fn remove_from_blacklist(&mut self, user_id: UserId) -> Bucket {
            let non_fungible_id: NonFungibleId = user_id.into();
            let user_badge_bucket = self
                .blacklisted_users
                .withdraw_non_fungible(non_fungible_id.clone());
            let manager = self.user_badge_manager();
            let user_badge = manager.get_non_fungible(&non_fungible_id);
            let user_data = user_badge.get_mutable_data::<UserMutableData>();
            manager.update_non_fungible_data(
                non_fungible_id,
                &UserMutableData {
                    is_blacklisted: false,
                    ..user_data
                },
            );
            emit_event("remove_from_blacklist", [("user_id", user_id.to_string())]);
            user_badge_bucket
        }

        pub fn get_user_data(&self, user_id: UserId) -> UserData {
            let badge = self.user_badge_manager().get_non_fungible(&user_id.into());
            badge.get_data()
        }

        pub fn set_user_wrapped_exchange_limit(&mut self, user_id: UserId, new_limit: Amount) {
            let mut badge = self.user_badge_manager().get_non_fungible(&user_id.into());
            let mut user_data = badge.get_mutable_data::<UserMutableData>();
            user_data.set_wrapped_exchange_limit(new_limit);
            badge.set_mutable_data(&user_data);
            emit_event(
                "set_user_wrapped_exchange_limit",
                [
                    ("user_id", user_id.to_string()),
                    ("limit", new_limit.to_string()),
                ],
            );
        }

        fn user_badge_manager(&self) -> ResourceManager {
            ResourceManager::get(self.user_auth_resource)
        }

        fn token_vault_manager(&self) -> ResourceManager {
            ResourceManager::get(self.token_vault.resource_address())
        }

        fn wrapped_token_mut(&mut self) -> &mut WrappedExchangeToken {
            self.wrapped_token
                .as_mut()
                .expect("Wrapped token is not enabled")
        }
    }
}
