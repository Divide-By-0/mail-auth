/*
 * Copyright (c) 2020-2023, Stalwart Labs Ltd.
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

use crate::{
    arc::Set,
    common::{
        crypto::{Algorithm, HashAlgorithm, SigningKey},
        verify::VerifySignature,
    },
    ArcOutput, DkimOutput, DkimResult, Error, Version,
};

pub mod builder;
pub mod canonicalize;
pub mod headers;
pub mod parse;
pub mod sign;
pub mod verify;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Canonicalization {
    Relaxed,
    Simple,
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct DkimSigner<T: SigningKey, State = NeedDomain> {
    _state: std::marker::PhantomData<State>,
    pub key: T,
    pub template: Signature,
}

pub struct NeedDomain;
pub struct NeedSelector;
pub struct NeedHeaders;
pub struct Done;

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct Signature {
    pub(crate) v: u32,
    pub(crate) a: Algorithm,
    pub(crate) d: String,
    pub(crate) s: String,
    pub(crate) b: Vec<u8>,
    pub(crate) bh: Vec<u8>,
    pub(crate) h: Vec<String>,
    pub(crate) z: Vec<String>,
    pub(crate) i: String,
    pub(crate) l: u64,
    pub(crate) x: u64,
    pub(crate) t: u64,
    pub(crate) r: bool,                      // RFC 6651
    pub(crate) atps: Option<String>,         // RFC 6541
    pub(crate) atpsh: Option<HashAlgorithm>, // RFC 6541
    pub(crate) ch: Canonicalization,
    pub(crate) cb: Canonicalization,
}

impl Default for Algorithm {
    fn default() -> Self {
        Algorithm::RsaSha256
    }
}

impl Default for Canonicalization {
    fn default() -> Self {
        Canonicalization::Relaxed
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DomainKeyReport {
    pub(crate) ra: String,
    pub(crate) rp: u8,
    pub(crate) rr: u8,
    pub(crate) rs: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Atps {
    pub(crate) v: Version,
    pub(crate) d: Option<String>,
}

pub(crate) const R_SVC_ALL: u64 = 0x04;
pub(crate) const R_SVC_EMAIL: u64 = 0x08;
pub(crate) const R_FLAG_TESTING: u64 = 0x10;
pub(crate) const R_FLAG_MATCH_DOMAIN: u64 = 0x20;

pub(crate) const RR_DNS: u8 = 0x01;
pub(crate) const RR_OTHER: u8 = 0x02;
pub(crate) const RR_POLICY: u8 = 0x04;
pub(crate) const RR_SIGNATURE: u8 = 0x08;
pub(crate) const RR_UNKNOWN_TAG: u8 = 0x10;
pub(crate) const RR_VERIFICATION: u8 = 0x20;
pub(crate) const RR_EXPIRATION: u8 = 0x40;

#[derive(Debug, PartialEq, Eq, Clone)]
#[repr(u64)]
pub(crate) enum Service {
    All = R_SVC_ALL,
    Email = R_SVC_EMAIL,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[repr(u64)]
pub(crate) enum Flag {
    Testing = R_FLAG_TESTING,
    MatchDomain = R_FLAG_MATCH_DOMAIN,
}

impl From<Flag> for u64 {
    fn from(v: Flag) -> Self {
        v as u64
    }
}

impl From<HashAlgorithm> for u64 {
    fn from(v: HashAlgorithm) -> Self {
        v as u64
    }
}

impl From<Service> for u64 {
    fn from(v: Service) -> Self {
        v as u64
    }
}

impl From<Algorithm> for HashAlgorithm {
    fn from(a: Algorithm) -> Self {
        match a {
            Algorithm::RsaSha256 | Algorithm::Ed25519Sha256 => HashAlgorithm::Sha256,
            Algorithm::RsaSha1 => HashAlgorithm::Sha1,
        }
    }
}

impl VerifySignature for Signature {
    fn signature(&self) -> &[u8] {
        &self.b
    }

    fn algorithm(&self) -> Algorithm {
        self.a
    }

    fn selector(&self) -> &str {
        &self.s
    }

    fn domain(&self) -> &str {
        &self.d
    }
}

impl Signature {
    pub fn identity(&self) -> &str {
        &self.i
    }

    pub fn body_hash(&self) -> &[u8] {
        &self.bh
    }
}

impl<'x> DkimOutput<'x> {
    pub(crate) fn pass() -> Self {
        DkimOutput {
            result: DkimResult::Pass,
            signature: None,
            report: None,
            is_atps: false,
        }
    }

    pub(crate) fn perm_err(err: Error) -> Self {
        DkimOutput {
            result: DkimResult::PermError(err),
            signature: None,
            report: None,
            is_atps: false,
        }
    }

    pub(crate) fn temp_err(err: Error) -> Self {
        DkimOutput {
            result: DkimResult::TempError(err),
            signature: None,
            report: None,
            is_atps: false,
        }
    }

    pub(crate) fn fail(err: Error) -> Self {
        DkimOutput {
            result: DkimResult::Fail(err),
            signature: None,
            report: None,
            is_atps: false,
        }
    }

    pub(crate) fn neutral(err: Error) -> Self {
        DkimOutput {
            result: DkimResult::Neutral(err),
            signature: None,
            report: None,
            is_atps: false,
        }
    }

    pub(crate) fn dns_error(err: Error) -> Self {
        if matches!(&err, Error::DnsError(_)) {
            DkimOutput::temp_err(err)
        } else {
            DkimOutput::perm_err(err)
        }
    }

    pub(crate) fn with_signature(mut self, signature: &'x Signature) -> Self {
        self.signature = signature.into();
        self
    }

    pub(crate) fn with_atps(mut self) -> Self {
        self.is_atps = true;
        self
    }

    pub fn result(&self) -> &DkimResult {
        &self.result
    }

    pub fn signature(&self) -> Option<&Signature> {
        self.signature
    }

    pub fn failure_report_addr(&self) -> Option<&str> {
        self.report.as_deref()
    }
}

impl<'x> ArcOutput<'x> {
    pub fn result(&self) -> &DkimResult {
        &self.result
    }

    pub fn sets(&self) -> &[Set] {
        &self.set
    }
}

impl From<Error> for DkimResult {
    fn from(err: Error) -> Self {
        if matches!(&err, Error::DnsError(_)) {
            DkimResult::TempError(err)
        } else {
            DkimResult::PermError(err)
        }
    }
}
