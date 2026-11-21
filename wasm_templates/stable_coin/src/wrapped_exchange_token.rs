// Copyright 2024 {{ authors }}
// SPDX-License-Identifier: BSD-3-Clause

use tari_template_lib::models::Vault;
use tari_template_lib::types::Amount;
use tari_template_lib::resource::ResourceManager;

#[derive(Debug, minicbor::Encode, minicbor::Decode, minicbor::CborLen)]
pub enum ExchangeFee {
    #[n(0)]
    Fixed(#[n(0)] Amount),
    #[n(1)]
    Percentage(#[n(0)] u64),
}

impl ExchangeFee {
    pub fn calculate_fee(&self, amount: Amount) -> Amount {
        match self {
            ExchangeFee::Fixed(fee) => *fee,
            ExchangeFee::Percentage(percentage) => {
                assert!(
                    *percentage <= 100,
                    "Exchange fee percentage cannot exceed 100%"
                );
                div_rounded(amount, (*percentage).into())
            }
        }
    }
}

#[derive(Debug, minicbor::Encode, minicbor::Decode, minicbor::CborLen)]
pub struct WrappedExchangeToken {
    #[n(0)]
    pub vault: Vault,
    #[n(1)]
    pub exchange_fee: ExchangeFee,
}

impl WrappedExchangeToken {

    pub(crate) fn get_resource_manager(&self) -> ResourceManager {
        self.vault.get_resource_manager()
    }

    pub fn vault(&self) -> &Vault {
        &self.vault
    }

    pub fn vault_mut(&mut self) -> &mut Vault {
        &mut self.vault
    }

    pub fn exchange_fee(&self) -> &ExchangeFee {
        &self.exchange_fee
    }
}

fn div_rounded<A: Into<Amount>>(v: A, p: A) -> Amount {
    let v = v.into();
    let p = p.into();
    let product = v
        .checked_mul(p)
        .expect("Overflow when calculating percentage fee");
    let b = product / Amount::ONE_HUNDRED;
    let remainder = product % Amount::ONE_HUNDRED;

    // If the decimal is greater or equal to 0.5, we round up
    if remainder >= Amount::from(50u64) {
        b + Amount::ONE
    } else {
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_div_rounded() {
        assert_eq!(div_rounded(0u64, 5), 0);
        assert_eq!(div_rounded(100u64, 0), 0);
        assert_eq!(div_rounded(100u64, 5), 5);
        assert_eq!(div_rounded(123u64, 5), 6);
        assert_eq!(div_rounded(130u64, 5), 7);
    }

    #[test]
    fn test_calculate_fee() {
        assert_eq!(ExchangeFee::Fixed(Amount::from(5u64)).calculate_fee(Amount::from(100u64)), 5);
        assert_eq!(ExchangeFee::Percentage(0).calculate_fee(Amount::from(100u64)), 0);
        assert_eq!(ExchangeFee::Percentage(1).calculate_fee(Amount::from(100u64)), 1);
        assert_eq!(ExchangeFee::Percentage(5).calculate_fee(Amount::from(130u64)), 7);
        assert_eq!(ExchangeFee::Percentage(100).calculate_fee(Amount::from(130u64)), 130);
    }

    #[test]
    #[should_panic(expected = "Exchange fee percentage cannot exceed 100%")]
    fn test_calculate_fee_rejects_percentage_over_100() {
        ExchangeFee::Percentage(101).calculate_fee(Amount::from(100u64));
    }
}
