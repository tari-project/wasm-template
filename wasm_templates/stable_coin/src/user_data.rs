// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::collections::BTreeMap;
use std::fmt::Display;
use tari_template_lib::models::{Amount, ComponentAddress, NonFungibleId, ResourceAddress, Vault};

#[derive(Clone, Debug, Copy, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct UserId(u64);

impl From<UserId> for NonFungibleId {
    fn from(value: UserId) -> Self {
        Self::from_u64(value.0)
    }
}

impl Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:0>19}", self.0)
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UserData {
    pub user_id: UserId,
    pub user_account: ComponentAddress,
    pub created_at: u64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UserMutableData {
    pub is_blacklisted: bool,
    pub wrapped_exchange_limit: Amount,
}

impl UserMutableData {
    pub fn set_wrapped_exchange_limit(&mut self, limit: Amount) -> &mut Self {
        self.wrapped_exchange_limit = limit;
        self
    }
}

impl Default for UserMutableData {
    fn default() -> Self {
        Self {
            is_blacklisted: false,
            wrapped_exchange_limit: 1000.into(),
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Account {
    pub vaults: BTreeMap<ResourceAddress, Vault>,
}

impl Account {
    pub fn get_vault(&self, address: &ResourceAddress) -> &Vault {
        self.vaults
            .get(address)
            .unwrap_or_else(|| panic!("Vault not found for address: {}", address))
    }
}
