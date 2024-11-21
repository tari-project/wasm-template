#[cfg(test)]
mod test {
    use tari_template_lib::args;
    use tari_template_lib::models::ComponentAddress;
    use tari_template_lib::prelude::*;
    use tari_template_test_tooling::TemplateTest;
    use tari_transaction::TransactionBuilder;

    #[test]
    fn test_nft() {
        let mut template_test = TemplateTest::new(vec!["."]);

        // Create an Account
        let (receiver_address, receiver_owner_proof, secret_key) =
            template_test.create_empty_account();

        // Create NFT component and resource
        let nft_component: ComponentAddress =
            template_test.call_function("{{ project-name | upper_camel_case }}Nft", "new", args![], vec![]);

        // Initially the total_supply of tokens is 0
        let total_supply: Amount =
            template_test.call_method(nft_component, "total_supply", args![], vec![]);
        assert_eq!(total_supply, Amount(0));

        let resource_address: ResourceAddress =
            template_test.call_method(nft_component, "get_resource_address", args![], vec![]);

        let result = template_test.try_execute(
            TransactionBuilder::new()
                .call_method(
                    nft_component,
                    "mint",
                    args![],
                )
                .put_last_instruction_output_on_workspace(
                    "new_nft"
                )
                .call_method(
                    receiver_address,
                    "deposit",
                    args![Variable("new_nft")],
                )
                .call_method(
                    receiver_address,
                    "balance",
                    args![resource_address],
                )
                .call_method(
                    nft_component,
                    "total_supply",
                    args![],
                )
                .build(),
            vec![receiver_owner_proof],
        ).unwrap();

        for log in result.finalize.logs {
            eprintln!("LOG: {}", log);
        }
        eprintln!("{:?}", result.finalize.execution_results);
        assert_eq!(
            result.finalize.execution_results[3].decode::<Amount>().unwrap(),
            Amount(1)
        );
        // After minting a token, the total_supply of tokens is 1
        assert_eq!(
            result.finalize.execution_results[4].decode::<Amount>().unwrap(),
            Amount(1)
        );
    }
}
