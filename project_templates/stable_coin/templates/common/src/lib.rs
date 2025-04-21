// Copyright 2023 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

mod issuer;

pub use issuer::*;
use tari_template_lib::auth::AccessRule;

use tari_template_lib::models::{Bucket, ResourceAddress};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CreateNewUserAccountResponse {
    pub token_resource: ResourceAddress,
    pub user_badge: Bucket,
    pub admin_only_access_rule: AccessRule,
}
