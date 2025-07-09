#[cfg(test)]
mod test {
    use tari_template_lib::prelude::*;
    use tari_template_test_tooling::TemplateTest;
    use tari_transaction::{args, Transaction};

    #[test]
    fn test_nft() {
        let mut test = TemplateTest::new(vec!["."]);

        // Create an Account
        let (receiver_address, _receiver_owner_proof, _secret_key) =
            test.create_empty_account();

        // Create NFT component and resource
        let (nft_component, resource_address): (ComponentAddress, ResourceAddress) =
            test.call_function("{{ project-name | upper_camel_case }}Nft", "new", args![], vec![test.owner_proof()]);

        // Initially the total_supply of tokens is 0
        let total_supply: Amount =
            test.call_method(nft_component, "total_supply", args![], vec![test.owner_proof()]);
        assert_eq!(total_supply, 0);

        let result = test.try_execute(
            Transaction::builder()
                .call_method(nft_component, "mint", args![])
                .put_last_instruction_output_on_workspace("new_nft")
                .call_method(receiver_address, "deposit", args![Workspace("new_nft")])
                .call_method(receiver_address, "balance", args![resource_address])
                .call_method(nft_component, "total_supply", args![])
                .build_and_seal(test.secret_key()),
            vec![test.owner_proof()],
        ).unwrap();

        for log in result.finalize.logs {
            eprintln!("LOG: {}", log);
        }
        eprintln!("{:?}", result.finalize.execution_results);
        assert_eq!(
            result.finalize.execution_results[3].decode::<Amount>().unwrap(),
            1
        );
        // After minting a token, the total_supply of tokens is 1
        assert_eq!(
            result.finalize.execution_results[4].decode::<Amount>().unwrap(),
            1
        );
    }
}
