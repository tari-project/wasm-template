// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use tari_template_lib::models::{ResourceAddress, Vault};
use tari_template_lib::types::Amount;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum ExchangeFee {
    Fixed(Amount),
    Percentage(u64),
}

impl ExchangeFee {
    pub fn calculate_fee(&self, amount: Amount) -> Amount {
        match self {
            ExchangeFee::Fixed(fee) => *fee,
            ExchangeFee::Percentage(percentage) => {
                let inv_perc = 100 / *percentage;
                div_rounded(amount, inv_perc.into()).try_into().unwrap()
            }
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct WrappedExchangeToken {
    pub vault: Vault,
    pub exchange_fee: ExchangeFee,
}

impl WrappedExchangeToken {
    pub(crate) fn resource_address(&self) -> ResourceAddress {
        self.vault.resource_address()
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
    // f and b are the division to 3 decimals
    let f = (v * Amount::ONE_THOUSAND) * p / Amount::ONE_HUNDRED;
    let b = v * p / Amount::ONE_HUNDRED;
    let c = f - (b * Amount::ONE_THOUSAND);

    // If the decimal is greater or equal to 0.5, we round up
    if c >= 500 {
        (f / Amount::ONE_THOUSAND) + Amount::ONE
    } else {
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_div_rounded() {
        assert_eq!(div_rounded(0, 5), 0);
        assert_eq!(div_rounded(100, 0), 0);
        assert_eq!(div_rounded(100, 5), 5);
        assert_eq!(div_rounded(123, 5), 6);
        assert_eq!(div_rounded(130, 5), 7);
    }
}
