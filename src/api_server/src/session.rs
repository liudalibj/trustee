// Copyright (c) 2022 by Rivos Inc.
// Licensed under the Apache License, Version 2.0, see LICENSE for details.
// SPDX-License-Identifier: Apache-2.0

use actix_web::cookie::{
    time::{Duration, OffsetDateTime},
    Cookie, Expiration,
};
use anyhow::Result;
use attestation_service::types::AttestationResults;
use kbs_types::{Request, Tee};
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use uuid::Uuid;

pub(crate) static KBS_SESSION_ID: &str = "kbs-session-id";
static SESSION_TIMEOUT: i64 = 5;

fn nonce() -> Result<String> {
    let mut nonce: Vec<u8> = vec![0; 32];

    thread_rng()
        .try_fill(&mut nonce[..])
        .map_err(anyhow::Error::from)?;

    Ok(base64::encode_config(&nonce, base64::STANDARD))
}

pub(crate) struct Session<'a> {
    cookie: Cookie<'a>,
    nonce: String,
    tee: Tee,
    tee_extra_params: Option<String>,
    attestation_results: Option<AttestationResults>,
}

impl<'a> Session<'a> {
    pub fn from_request(req: &Request) -> Result<Self> {
        let id = Uuid::new_v4().as_simple().to_string();
        let tee_extra_params = if req.extra_params.is_empty() {
            None
        } else {
            Some(req.extra_params.clone())
        };

        let cookie = Cookie::build(KBS_SESSION_ID, id)
            .expires(OffsetDateTime::now_utc() + Duration::minutes(SESSION_TIMEOUT))
            .finish();

        Ok(Session {
            cookie,
            nonce: nonce()?,
            tee: req.tee.clone(),
            tee_extra_params,
            attestation_results: None,
        })
    }

    pub fn id(&self) -> &str {
        self.cookie.value()
    }

    pub fn cookie(&self) -> Cookie {
        self.cookie.clone()
    }

    pub fn nonce(&self) -> &str {
        &self.nonce
    }

    pub fn tee(&self) -> Tee {
        self.tee.clone()
    }

    pub fn is_authenticated(&self) -> bool {
        self.attestation_results
            .as_ref()
            .map_or(false, |a| a.allow())
    }

    pub fn is_expired(&self) -> bool {
        if let Some(Expiration::DateTime(time)) = self.cookie.expires() {
            return OffsetDateTime::now_utc() > time;
        }

        false
    }

    pub fn is_valid(&self) -> bool {
        self.is_authenticated() && !self.is_expired()
    }
}

pub(crate) struct SessionMap<'a> {
    pub sessions: RwLock<HashMap<String, Arc<Mutex<Session<'a>>>>>,
}

impl<'a> SessionMap<'a> {
    pub fn new() -> Self {
        SessionMap {
            sessions: RwLock::new(HashMap::new()),
        }
    }
}
