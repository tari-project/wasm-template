// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use tari_template_lib::models::{Amount, ResourceAddress, Vault};

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
                div_rounded(amount.as_u64_checked().unwrap(), inv_perc)
                    .try_into()
                    .unwrap()
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

fn div_rounded(v: u64, p: u64) -> u64 {
    // f and b are the division to 3 decimals
    let f = (v * 1000) * p / 100;
    let b = v * p / 100;
    let c = f - (b * 1000);

    // If the decimal is greater or equal to 0.5, we round up
    if c >= 500 {
        (f / 1000) + 1
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
