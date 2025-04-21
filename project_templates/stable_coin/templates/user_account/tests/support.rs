// Copyright 2023 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use tari_template_lib::args;
use tari_template_lib::models::{
    Amount, Bucket, ComponentAddress, Metadata, NonFungibleAddress, ResourceAddress, VaultId,
};
use tari_template_lib::prelude::{RistrettoPublicKeyBytes, TemplateAddress};
use tari_template_test_tooling::crypto::RistrettoSecretKey;
use tari_template_test_tooling::support::confidential::generate_withdraw_proof;
use tari_template_test_tooling::{support::confidential, TemplateTest};
use tari_transaction::Transaction;

pub struct UserAccountTest {
    pub test: TemplateTest,
    pub stable_coin_issuer_component: ComponentAddress,
    pub user_account_template: TemplateAddress,
    pub admin_account: ComponentAddress,
    pub admin_proof: NonFungibleAddress,
    pub admin_key: RistrettoSecretKey,
    pub admin_badge_resource: ResourceAddress,
    pub user_badge_resource: ResourceAddress,
    pub token_resource: ResourceAddress,
    pub supply_output_mask: RistrettoSecretKey,
    pub supply_amount: Amount,
}

impl UserAccountTest {
    pub fn new() -> Self {
        const INITIAL_SUPPLY: Amount = Amount(1_000_000_000);
        let mut test = TemplateTest::new(["./", "../issuer"]);
        let (admin_account, admin_proof, admin_key) = test.create_owned_account();
        let issuer_template = test.get_template_address("PrivateStableCoinIssuer");
        let user_account_template = test.get_template_address("PrivateStableCoinUserAccount");
        let mut metadata = Metadata::new();
        metadata
            .insert("provider_name", "Stable coinz 4 U")
            .insert("collateralized_by", "Z$")
            .insert("issuing_authority", "Bank of Silly Walks")
            .insert("issued_at", "2023-01-01");

        let (initial_supply, initial_supply_mask, _) =
            confidential::generate_confidential_proof(INITIAL_SUPPLY, None);

        let result = test.execute_expect_success(
            Transaction::builder()
                .call_function(
                    issuer_template,
                    "instantiate",
                    args![initial_supply, "SC4U", user_account_template, metadata],
                )
                .put_last_instruction_output_on_workspace("ret")
                .call_method(admin_account, "deposit", args![Workspace("ret.1")])
                .build_and_seal(&admin_key),
            vec![admin_proof.clone()],
        );

        let (stable_coin_issuer_component, _) = result.finalize.execution_results[0]
            .decode::<(ComponentAddress, Bucket)>()
            .unwrap();

        let indexed = test
            .read_only_state_store()
            .inspect_component(stable_coin_issuer_component)
            .unwrap();

        let token_vault = indexed
            .get_value("$.token_vault")
            .unwrap()
            .expect("user_badge_resource not found");
        let user_badge_resource = indexed
            .get_value("$.user_auth_resource")
            .unwrap()
            .expect("user_auth_resource not found");
        let admin_badge_resource = indexed
            .get_value("$.admin_auth_resource")
            .unwrap()
            .expect("admin_auth_resource not found");

        let vault = test
            .read_only_state_store()
            .get_vault(&token_vault)
            .unwrap();
        let token_resource = *vault.resource_address();

        UserAccountTest {
            test,
            stable_coin_issuer_component,
            user_account_template,
            admin_account,
            admin_proof,
            admin_key,
            admin_badge_resource,
            user_badge_resource,
            token_resource,
            supply_output_mask: initial_supply_mask,
            supply_amount: INITIAL_SUPPLY,
        }
    }

    pub fn open_accounts(
        &mut self,
        public_keys: Vec<RistrettoPublicKeyBytes>,
    ) -> Vec<ComponentAddress> {
        // Open account
        let mut builder = Transaction::builder()
            .create_proof(self.admin_account, self.admin_badge_resource)
            .put_last_instruction_output_on_workspace("proof");

        for pk in &public_keys {
            builder = builder.call_function(
                self.user_account_template,
                "create",
                args![self.stable_coin_issuer_component, pk, Workspace("proof")],
            )
        }

        let result = self.test.execute_expect_success(
            builder
                .drop_all_proofs_in_workspace()
                .build_and_seal(&self.admin_key),
            vec![self.admin_proof.clone()],
        );

        result.finalize.execution_results[2..2 + public_keys.len()]
            .iter()
            .map(|result| result.decode().unwrap())
            .collect()
    }

    pub fn fund_account(
        &mut self,
        account: ComponentAddress,
        amount: Amount,
    ) -> RistrettoSecretKey {
        // Current strategy in the wallet is to generate a DH key for the mask (similar to one-sided stealth)
        // In a test we just use a random mask
        let fund_proof = generate_withdraw_proof(
            &self.supply_output_mask,
            amount,
            Some(
                self.supply_amount
                    .checked_sub(amount)
                    .expect("Not enough supply"),
            ),
            Amount::zero(),
        );

        // Fund account
        self.test.execute_expect_success(
            Transaction::builder()
                .create_proof(self.admin_account, self.admin_badge_resource)
                .put_last_instruction_output_on_workspace("proof")
                .call_method(
                    self.stable_coin_issuer_component,
                    "withdraw_confidential",
                    args![fund_proof.proof, "Funding user account",],
                )
                .put_last_instruction_output_on_workspace("funds")
                .call_method(
                    account,
                    "deposit",
                    args![Workspace("proof"), Workspace("funds")],
                )
                .drop_all_proofs_in_workspace()
                .build_and_seal(&self.admin_key),
            vec![self.admin_proof.clone()],
        );

        self.supply_output_mask = fund_proof.change_mask.unwrap();
        self.supply_amount -= amount;
        fund_proof.output_mask
    }

    pub fn add_account_to_deny_list(
        &mut self,
        pk: RistrettoPublicKeyBytes,
        account: ComponentAddress,
        vault_id: VaultId,
    ) {
        // Fund account
        self.test.execute_expect_success(
            Transaction::builder()
                .create_proof(self.admin_account, self.admin_badge_resource)
                .put_last_instruction_output_on_workspace("proof")
                .call_method(
                    self.stable_coin_issuer_component,
                    "add_user_to_deny_list",
                    // TODO: right now we need to pass both in
                    args![Workspace("proof"), pk, account, vault_id],
                )
                .drop_all_proofs_in_workspace()
                .build_and_seal(&self.admin_key),
            vec![self.admin_proof.clone()],
        );
    }

    pub fn remove_account_from_deny_list(&mut self, pk: RistrettoPublicKeyBytes) {
        // Fund account
        self.test.execute_expect_success(
            Transaction::builder()
                .create_proof(self.admin_account, self.admin_badge_resource)
                .put_last_instruction_output_on_workspace("proof")
                .call_method(
                    self.stable_coin_issuer_component,
                    "remove_user_from_deny_list",
                    // TODO: right now we need to pass both in
                    args![Workspace("proof"), pk],
                )
                .drop_all_proofs_in_workspace()
                .build_and_seal(&self.admin_key),
            vec![self.admin_proof.clone()],
        );
    }
}
