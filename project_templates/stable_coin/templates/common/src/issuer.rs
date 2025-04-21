// Copyright 2023 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use crate::CreateNewUserAccountResponse;
use tari_template_lib::prelude::*;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct IssuerApi {
    component_address: ComponentAddress,
}

impl IssuerApi {
    pub fn new(component_address: ComponentAddress) -> Self {
        Self { component_address }
    }

    pub fn check_transfer(&self, proof: Proof, destination_account: ComponentAddress) {
        self.component_manager()
            .call("check_transfer", args![proof, destination_account])
    }

    pub fn check_deposit(&self, proof: Proof) {
        self.component_manager().call("check_deposit", args![proof])
    }

    pub fn create_user_account(
        &self,
        proof: Proof,
        user_public_key: RistrettoPublicKeyBytes,
    ) -> CreateNewUserAccountResponse {
        self.component_manager()
            .call("create_user_account", args![proof, user_public_key])
    }

    fn component_manager(&self) -> ComponentManager {
        ComponentManager::get(self.component_address)
    }
}
