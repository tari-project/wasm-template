#[cfg(test)]
mod test {
    use tari_engine_types::instruction::Instruction;
    use tari_template_lib::args;
    use tari_template_lib::models::ComponentAddress;
    use tari_template_lib::prelude::*;
    use tari_template_test_tooling::TemplateTest;

    #[test]
    fn test_nft() {
        let template_test = TemplateTest::new(vec!["../package"]);

        // Create an Account
        let receiver_address: ComponentAddress =
            template_test.call_function("Account", "new", args![]);

        // Create NFT component and resource
        let nft_component: ComponentAddress =
            template_test.call_function("Nft", "new", args![]);

        // Initially the total_supply of tokens is 0
        let total_supply: Amount =
            template_test.call_method(nft_component, "total_supply", args![]);
        assert_eq!(total_supply, Amount(0));

        let resource_address: ResourceAddress =
            template_test.call_method(nft_component, "get_resource_address", args![]);

        // Mint token and transfer to the account
        let result = template_test.try_execute(vec![
            Instruction::CallMethod {
                component_address: nft_component,
                method: "mint".to_string(),
                args: args![],
            },
            Instruction::PutLastInstructionOutputOnWorkspace {
                key: b"new_nft".to_vec(),
            },
            Instruction::CallMethod {
                component_address: receiver_address,
                method: "deposit".to_string(),
                args: args![Variable("new_nft")],
            },
            Instruction::CallMethod {
                component_address: receiver_address,
                method: "balance".to_string(),
                args: args![resource_address],
            },
            Instruction::CallMethod {
                component_address: nft_component,
                method: "total_supply".to_string(),
                args: args![],
            },
        ]).unwrap();

        for log in result.logs {
            eprintln!("LOG: {}", log);
        }
        eprintln!("{:?}", result.execution_results);
        assert_eq!(
            result.execution_results[3].decode::<Amount>().unwrap(),
            Amount(1)
        );
        // After minting a token, the total_supply of tokens is 1
        assert_eq!(
            result.execution_results[4].decode::<Amount>().unwrap(),
            Amount(1)
        );
    }
}
