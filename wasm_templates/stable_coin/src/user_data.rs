// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::fmt::Display;
use tari_template_lib::types::{ComponentAddress, NonFungibleId};
use tari_template_lib::types::Amount;

#[derive(Clone, Debug, Copy)]
pub struct UserId(u64);

// UserId is a transparent newtype: on the wire it's just a CBOR u64, matching what manifests
// emit for `123u64` literals. The derive macros would wrap the inner u64 in a single-element
// array; we want bare u64 so the manifest path round-trips.
impl<C> minicbor::Encode<C> for UserId {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.u64(self.0)?;
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for UserId {
    fn decode(d: &mut minicbor::Decoder<'b>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        Ok(UserId(d.u64()?))
    }
}

impl<C> minicbor::CborLen<C> for UserId {
    fn cbor_len(&self, ctx: &mut C) -> usize {
        self.0.cbor_len(ctx)
    }
}

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

#[derive(Clone, Debug, minicbor::Encode, minicbor::Decode, minicbor::CborLen)]
pub struct UserData {
    #[n(0)]
    pub user_id: UserId,
    #[n(1)]
    pub user_account: ComponentAddress,
    #[n(2)]
    pub created_at: u64,
}

#[derive(Clone, Debug, minicbor::Encode, minicbor::Decode, minicbor::CborLen)]
pub struct UserMutableData {
    #[n(0)]
    pub is_blacklisted: bool,
    #[n(1)]
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
            wrapped_exchange_limit: 1000u64.into(),
        }
    }
}
