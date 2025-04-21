// Copyright 2023 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use tari_template_lib::args;
use tari_template_lib::models::{
    Amount, Bucket, ComponentAddress, Metadata, NonFungibleAddress, ResourceAddress,
};
use tari_template_lib::prelude::TemplateAddress;
use tari_template_test_tooling::crypto::RistrettoSecretKey;
use tari_template_test_tooling::{support::confidential, TemplateTest};
use tari_transaction::Transaction;

pub struct IssuerTest {
    pub test: TemplateTest,
    pub stable_coin_issuer_component: ComponentAddress,
    pub user_account_template: TemplateAddress,
    pub admin_account: ComponentAddress,
    pub admin_proof: NonFungibleAddress,
    pub admin_key: RistrettoSecretKey,
    pub admin_badge_resource: ResourceAddress,
    pub user_badge_resource: ResourceAddress,
    pub token_resource: ResourceAddress,
    pub initial_supply_mask: RistrettoSecretKey,
}

impl IssuerTest {
    pub fn new() -> Self {
        let mut test = TemplateTest::new(["../user_account", "."]);
        let (admin_account, admin_proof, admin_key) = test.create_funded_account();
        let issuer_template = test.get_template_address("PrivateStableCoinIssuer");
        let user_account_template = test.get_template_address("PrivateStableCoinUserAccount");
        let mut metadata = Metadata::new();
        metadata
            .insert("provider_name", "Stable coinz 4 U")
            .insert("collateralized_by", "Z$")
            .insert("issuing_authority", "Bank of Silly Walks")
            .insert("issued_at", "2023-01-01");

        let (output, initial_supply_mask, _) =
            confidential::generate_confidential_proof(Amount(1_000), None);

        let result = test.execute_expect_success(
            Transaction::builder()
                .call_function(
                    issuer_template,
                    "instantiate",
                    args![output, "SC4U", user_account_template, metadata],
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

        IssuerTest {
            test,
            stable_coin_issuer_component,
            user_account_template,
            admin_account,
            admin_proof,
            admin_key,
            admin_badge_resource,
            user_badge_resource,
            token_resource,
            initial_supply_mask,
        }
    }
}

impl Default for IssuerTest {
    fn default() -> Self {
        Self::new()
    }
}
