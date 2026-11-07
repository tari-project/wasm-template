use std::collections::BTreeMap;
use tari_template_lib::prelude::*;

#[template]
mod asd {
    use super::*;
    use tari_template_lib::types::precision::PrecisionAmount;

    // Constant product AMM
    pub struct {{ project-name | upper_camel_case }}Pool {
        pools: BTreeMap<ResourceAddress, Vault>,
        lp_resource: ResourceManager,
        fee: u16,
    }

    impl {{ project-name | upper_camel_case }}Pool {
        // Initialises a new pool component for for the pool A - B
        // the fees is represented as a per-mil quantity (e.g. "1" represents "0.1%")
        pub fn new(a_addr: ResourceAddress, b_addr: ResourceAddress, fee: u16) -> Self {
            // check that the the resource pair is correct
            assert_ne!(a_addr, b_addr, "The resources of the pair must be different");
            Self::check_resource_is_fungible(a_addr);
            Self::check_resource_is_fungible(b_addr);

            // the fee represents a percentage, so it must be between 0 and 100
            let valid_fee_range = 0..1000;
            assert!(valid_fee_range.contains(&fee), "Invalid fee {}", fee);

            // create the vaults to store the funds
            let mut pools = BTreeMap::new();
            pools.insert(a_addr, Vault::new_empty(a_addr));
            pools.insert(b_addr, Vault::new_empty(b_addr));

            // create the lp resource
            // TODO: add lp resource minting/burning security, only this component should be allowed
            let lp_resource = ResourceBuilder::public_fungible().with_token_symbol("LP").build();

            Self {
                pools,
                lp_resource: ResourceManager::get(lp_resource),
                fee,
            }
        }

        // swap A tokens for B tokens or vice versa
        pub fn swap(&mut self, input_bucket: Bucket, output_resource: ResourceAddress) -> Bucket {
            // check that the parameters are correct
            let input_resource = input_bucket.resource_address();
            self.check_pool_resources(input_resource, output_resource);

            // get the data needed to calculate the pool rebalancing
            let input_pool_balance = self.get_pool_balance(input_resource);
            let output_pool_balance = self.get_pool_balance(output_resource);

            // check that the pools are not empty, to prevent division by 0 errors later
            assert!(
                !input_pool_balance.is_zero(),
                "The pool for resource '{}' is empty",
                input_resource
            );
            assert!(
                !output_pool_balance.is_zero(),
                "The pool for resource '{}' is empty",
                output_resource
            );

            // apply the fee to the input bucket
            // so the user will get a lesser amount of tokens than the theoretical (for the gain of the LP holders)
            let input_bucket_balance = input_bucket.amount();
            assert!(!input_bucket_balance.is_zero(), "Swap input amount must be greater than zero");
            let fee_amount = input_bucket_balance
                .into_precision_amount()
                .checked_mul(Amount::from(self.fee).into_precision_amount())
                .and_then(|num| num.checked_div_ceil(Amount::from(1000u64).into_precision_amount()))
                .expect("Overflow when calculating swap fee");
            let fee_amount = Amount::try_from(fee_amount).expect("Swap fee conversion failed");
            let effective_input_balance = input_bucket_balance - fee_amount;

            // recalculate the new vault balances for the swap
            // constant product AMM formula is "k = a * b"
            // so the new output vault balance should be "b = ceil(k / a)" to preserve the pool invariant
            let k = input_pool_balance
                .into_precision_amount()
                .checked_mul(output_pool_balance.into_precision_amount())
                .expect("Overflow when calculating pool invariant k");
            let new_input_pool_balance = input_pool_balance + effective_input_balance;
            let new_output_pool_balance = k
                .checked_div_ceil(new_input_pool_balance.into_precision_amount())
                .expect("Overflow when calculating new output pool balance");
            let new_output_pool_balance =
                Amount::try_from(new_output_pool_balance).expect("New output pool balance conversion failed");

            // calculate the amount of output tokens to return to the user
            let output_bucket_amount = output_pool_balance - new_output_pool_balance;
            assert!(
                output_bucket_amount.is_positive(),
                "Swap input amount too small to receive any output tokens"
            );

            // perform the swap
            self.pools
                .get_mut(&input_resource)
                .unwrap()
                .deposit(input_bucket);
            self.pools
                .get_mut(&output_resource)
                .unwrap()
                .withdraw(output_bucket_amount)
        }

        /// Adds liquidity to the pool, minting LP tokens in return.
        ///
        /// Mints the most LP both buckets can pay for and takes from each bucket the share of its reserve that LP is
        /// worth, rounded up in favour of the pool. The rest of both buckets is returned as change.
        /// Returns `(lp_tokens, change_a, change_b)`, where the change buckets correspond to `a_bucket`/`b_bucket`;
        /// either may be empty.
        pub fn add_liquidity(&mut self, mut a_bucket: Bucket, mut b_bucket: Bucket) -> (Bucket, Bucket, Bucket) {
            const OVERFLOW_MSG: &str = "Overflow when calculating LP token mint amount";

            // check that the buckets are correct
            let a_resource = a_bucket.resource_address();
            let b_resource = b_bucket.resource_address();
            self.check_pool_resources(a_resource, b_resource);

            // Compute in 192-bit precision: `Amount` products of two large balances can overflow
            let a_amount = a_bucket.amount().into_precision_amount();
            let b_amount = b_bucket.amount().into_precision_amount();
            assert!(
                a_amount.is_positive() && b_amount.is_positive(),
                "Cannot add zero liquidity"
            );

            let a_reserve = self.get_pool_balance(a_resource).into_precision_amount();
            let b_reserve = self.get_pool_balance(b_resource).into_precision_amount();
            let lp_supply = self.lp_total_supply().into_precision_amount();

            let (lp_mint_amount, a_contribution, b_contribution) = if lp_supply.is_zero() {
                // Initial liquidity: there is no existing price to preserve, so the first provider sets it. Take the
                // full contribution and mint LP equal to the geometric mean `sqrt(a * b)`.
                assert!(
                    a_reserve.is_zero() && b_reserve.is_zero(),
                    "Inconsistent pool state: zero LP supply with non-zero reserve"
                );
                let mint = a_amount
                    .checked_mul(b_amount)
                    .and_then(|product| product.checked_sqrt())
                    .expect(OVERFLOW_MSG);
                (mint, a_amount, b_amount)
            } else {
                assert!(
                    a_reserve.is_positive() && b_reserve.is_positive(),
                    "Inconsistent pool state: non-zero LP supply with zero reserve"
                );
                // Multiply before dividing: dividing first truncates the ratio to zero whenever a contribution is
                // smaller than its reserve. `mint <= amount * lp_supply / reserve` keeps each rounded-up take within
                // its bucket.
                let mint_for = |amount: PrecisionAmount, reserve: PrecisionAmount| {
                    amount
                        .checked_mul(lp_supply)
                        .and_then(|num| num.checked_div(reserve))
                        .expect(OVERFLOW_MSG)
                };
                let take_for = |mint: PrecisionAmount, reserve: PrecisionAmount| {
                    mint.checked_mul(reserve)
                        .and_then(|num| num.checked_div_ceil(lp_supply))
                        .expect(OVERFLOW_MSG)
                };

                let mint = mint_for(a_amount, a_reserve).min(mint_for(b_amount, b_reserve));
                (mint, take_for(mint, a_reserve), take_for(mint, b_reserve))
            };

            let lp_mint_amount = Amount::try_from(lp_mint_amount).expect("LP mint amount conversion failed");
            let a_contribution = Amount::try_from(a_contribution).expect("A contribution conversion failed");
            let b_contribution = Amount::try_from(b_contribution).expect("B contribution conversion failed");

            // Reject dust contributions that round down to a zero contribution or zero mint
            assert!(
                a_contribution.is_positive() && b_contribution.is_positive() && lp_mint_amount.is_positive(),
                "Contribution too small relative to pool reserves to mint any LP tokens"
            );

            // add the liquidity to the pool
            let contributed_a = a_bucket.take(a_contribution);
            let contributed_b = b_bucket.take(b_contribution);
            self.pools.get_mut(&a_resource).unwrap().deposit(contributed_a);
            self.pools.get_mut(&b_resource).unwrap().deposit(contributed_b);

            // mint the new lp tokens and return them with the unused remainder of each bucket
            let lp_tokens = self.lp_resource.mint_fungible(lp_mint_amount);
            (lp_tokens, a_bucket, b_bucket)
        }

        pub fn remove_liquidity(&mut self, lp_bucket: Bucket) -> (Bucket, Bucket) {
            assert_eq!(lp_bucket.resource_address(), self.lp_resource.resource_address(), "Invalid LP resource");

            // get the pool information
            let a_resource = self.get_a_resource();
            let a_balance = self.get_pool_balance(a_resource);
            let b_resource = self.get_b_resource();
            let b_balance = self.get_pool_balance(b_resource);

            // calculate the amount of tokens to take from each pool
            let total_lp = self.lp_total_supply();
            assert!(!total_lp.is_zero(), "LP total supply is zero");
            let lp_amount = lp_bucket.amount();
            let a_amount = Self::pro_rata_share(lp_amount, a_balance, total_lp);
            let b_amount = Self::pro_rata_share(lp_amount, b_balance, total_lp);

            // A redemption that rounds down to zero of either reserve would burn LP for nothing
            assert!(
                a_amount.is_positive() && b_amount.is_positive(),
                "Redemption amount too small to withdraw any tokens from the pool"
            );

            // burn the LP tokens
            lp_bucket.burn();

            // return the pool tokens
            let a_bucket = self.pools.get_mut(&a_resource).unwrap().withdraw(a_amount);
            let b_bucket = self.pools.get_mut(&b_resource).unwrap().withdraw(b_amount);
            (a_bucket, b_bucket)
        }

        pub fn get_a_resource(&self) -> ResourceAddress {
            *self.pools.keys().nth(0).unwrap()
        }

        pub fn get_b_resource(&self) -> ResourceAddress {
            *self.pools.keys().nth(1).unwrap()
        }

        pub fn get_pool_balances(&self) -> BTreeMap<ResourceAddress, Amount> {
            let mut balances = BTreeMap::new();

            for (resource, vault) in &self.pools {
                balances.insert(resource.clone(), vault.balance());
            }

            balances
        }

        pub fn get_pool_balance(&self, resource_address: ResourceAddress) -> Amount {
            let vault = self
                .pools
                .get(&resource_address)
                .unwrap_or_else(|| panic!("Resource {} is not in the pool", resource_address));
            vault.balance()
        }

        /// `amount * reserve / total`, rounded down in favour of the pool. Computed in 192-bit precision so that
        /// the product cannot overflow.
        fn pro_rata_share(amount: Amount, reserve: Amount, total: Amount) -> Amount {
            let share = amount
                .into_precision_amount()
                .checked_mul(reserve.into_precision_amount())
                .and_then(|num| num.checked_div(total.into_precision_amount()))
                .expect("Overflow when calculating redemption amount");
            Amount::try_from(share).expect("Redemption amount conversion failed")
        }

        pub fn lp_total_supply(&self) -> Amount {
            self.lp_resource.total_supply()
        }

        pub fn fee(&self) -> u16 {
            self.fee
        }

        fn check_pool_resources(&self, a_resource: ResourceAddress, b_resource: ResourceAddress) {
            assert!(
                a_resource != b_resource,
                "The resource addresses are the same"
            );
            assert!(
                self.pools.contains_key(&a_resource),
                "The resource {} is not in the pool",
                a_resource
            );
            assert!(
                self.pools.contains_key(&b_resource),
                "The resource {} is not in the pool",
                b_resource
            );
        }

        fn check_resource_is_fungible(resource: ResourceAddress) {
            assert_eq!(
                ResourceManager::get(resource).resource_type(),
                ResourceType::Fungible,
                "Resource {} is not fungible",
                resource
            );
        }
    }
}
