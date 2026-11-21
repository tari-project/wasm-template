use std::collections::BTreeMap;
use tari_engine_types::virtual_substate::{VirtualSubstate, VirtualSubstateId};
use tari_template_lib::prelude::TARI_TOKEN;
use tari_template_lib::types::{Amount, ComponentAddress, TemplateAddress};
use tari_template_test_tooling::support::assert_error::assert_reject_reason;
use tari_template_test_tooling::transaction::args;
use tari_template_test_tooling::TemplateTest;

#[test]
fn auction_index_allows_external_callers_and_rejects_zero_or_overflow_epoch_period() {
    let mut test = TemplateTest::my_crate();
    let index_template = test.get_template_address("AuctionIndex");

    let (_deployer_account, deployer_token, deployer_key) = test.create_funded_account();
    let (seller_account, seller_token, seller_key) = test.create_funded_account();

    test.execute_expect_success(
        test.transaction()
            .call_function(index_template, "new", args![TemplateAddress::default()])
            .build_and_seal(&deployer_key),
        vec![deployer_token],
    );
    let index_component = test.expect_return::<ComponentAddress>(0);

    // A non-deployer account can query `get_auctions` because `AuctionIndex::new` registers
    // `AccessRules::allow_all()` on the created component.
    test.execute_expect_success(
        test.transaction()
            .call_method(index_component, "get_auctions", args![])
            .build_and_seal(&seller_key),
        vec![seller_token.clone()],
    );
    let auctions = test.expect_return::<BTreeMap<u64, Vec<ComponentAddress>>>(0);
    assert!(auctions.is_empty());

    // `epoch_period == 0` is rejected before invoking the auction template.
    let reason = test.execute_expect_failure(
        test.transaction()
            .call_method(seller_account, "withdraw", args![TARI_TOKEN, Amount::new(1)])
            .put_last_instruction_output_on_workspace("bucket")
            .call_method(
                index_component,
                "create_auction",
                args![
                    Workspace("bucket"),
                    seller_account,
                    None::<Amount>,
                    None::<Amount>,
                    0u64
                ],
            )
            .build_and_seal(&seller_key),
        vec![seller_token.clone()],
    );
    assert_reject_reason(reason, "Invalid auction period");

    // Advance current_epoch > 0 and verify `epoch_period = u64::MAX` fails with "Epoch overflow".
    test.set_virtual_substate(
        VirtualSubstateId::CurrentEpoch,
        VirtualSubstate::CurrentEpoch(1),
    );
    let reason = test.execute_expect_failure(
        test.transaction()
            .call_method(seller_account, "withdraw", args![TARI_TOKEN, Amount::new(1)])
            .put_last_instruction_output_on_workspace("bucket")
            .call_method(
                index_component,
                "create_auction",
                args![
                    Workspace("bucket"),
                    seller_account,
                    None::<Amount>,
                    None::<Amount>,
                    u64::MAX
                ],
            )
            .build_and_seal(&seller_key),
        vec![seller_token],
    );
    assert_reject_reason(reason, "Epoch overflow");
}
