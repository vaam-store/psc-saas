#![deny(clippy::all)]
#![forbid(unsafe_code)]

//! Unified Provider Gateway service.
//!
//! This crate implements the core logic for the Unified Provider Gateway,
//! abstracting interactions with various mobile money providers.

use async_trait::async_trait;
use psc_error::{Error, Result};
use psc_provider::{
    pb::{
        balance::v1::{Balance, GetBalanceRequest},
        common::v1::{Id, Money, Timestamp},
        journal::v1::{JournalEntry, PostJournalRequest},
        payment::v1::{CreatePaymentRequest, Payment, PaymentStatus},
        payout::v1::{CreatePayoutRequest, Payout, PayoutStatus},
    },
    Ctx, Provider,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use cuid::cuid2;
use time;
use std::str::FromStr;
use rust_decimal::prelude::ToPrimitive;
// Idempotency and Redis caching are currently disabled until types implement serde
use nats::asynk::Connection as NatsClient; // NATS client

/// Configuration for the MTN Sandbox Provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MtnSandboxConfig {
    pub base_url: String,
    pub api_key: String, // X-Reference-Id for MTN
    pub target_environment: String, // X-Target-Environment
    pub webhook_secret: String, // Secret for verifying webhooks
    pub redis_url: String, // Redis URL for idempotency and caching
    pub nats_url: String, // NATS URL for event bus
    pub cache_ttl_seconds: u64, // TTL for cached items
}

/// Adapter for the MTN Sandbox environment implementing the Provider trait.
#[derive(Debug, Clone)]
pub struct MtnSandboxAdapter {
    config: MtnSandboxConfig,
    client: Client,
    collection_cfg: psc_mtn_collection::apis::configuration::Configuration,
    disbursement_cfg: psc_mtn_disbursement::apis::configuration::Configuration,
    remittance_cfg: psc_mtn_remittance::apis::configuration::Configuration,
    sandbox_provisioning_cfg: psc_mtn_sandbox_provisioning::apis::configuration::Configuration,
    nats_client: NatsClient,
}

impl MtnSandboxAdapter {
    pub async fn new(config: MtnSandboxConfig) -> Self {
        let reqwest_client = Client::new();
        let collection_config = psc_mtn_collection::apis::configuration::Configuration {
            base_path: config.base_url.clone(),
            user_agent: Some("psc-provider-gateway".to_string()),
            client: reqwest_client.clone(),
            // No API key directly here, it's passed as header
            ..Default::default()
        };
        let disbursement_config = psc_mtn_disbursement::apis::configuration::Configuration {
            base_path: config.base_url.clone(),
            user_agent: Some("psc-provider-gateway".to_string()),
            client: reqwest_client.clone(),
            // No API key directly here, it's passed as header
            ..Default::default()
        };
        let remittance_config = psc_mtn_remittance::apis::configuration::Configuration {
            base_path: config.base_url.clone(),
            user_agent: Some("psc-provider-gateway".to_string()),
            client: reqwest_client.clone(),
            // No API key directly here, it's passed as header
            ..Default::default()
        };
        let sandbox_provisioning_config = psc_mtn_sandbox_provisioning::apis::configuration::Configuration {
            base_path: config.base_url.clone(),
            user_agent: Some("psc-provider-gateway".to_string()),
            client: reqwest_client.clone(),
            ..Default::default()
        };

        let nats_client = nats::asynk::connect(&config.nats_url)
            .await
            .expect("Failed to connect to NATS server"); // TODO: Handle error properly

        MtnSandboxAdapter {
            config,
            client: reqwest_client,
            collection_cfg: collection_config,
            disbursement_cfg: disbursement_config,
            remittance_cfg: remittance_config,
            sandbox_provisioning_cfg: sandbox_provisioning_config,
            nats_client,
        }
    }

    /// Helper to map MTN Collection API errors to our unified Error type.
    fn map_mtn_collection_error<T>(e: psc_mtn_collection::apis::Error<T>) -> Error {
        match e {
            psc_mtn_collection::apis::Error::ResponseError(response_error) => {
                let status_code = response_error.status.as_u16();
                let content = response_error.content;

                // Try to parse MTN's ErrorReason structure
                if let Ok(error_reason) = serde_json::from_slice::<MtnErrorReason>(content.as_bytes()) {
                    Error::Provider {
                        code: error_reason.code.unwrap_or_else(|| "UNKNOWN_MTN_COLLECTION_ERROR_CODE".to_string()),
                        message: error_reason.message.unwrap_or_else(|| format!("MTN Collection API error (HTTP {}): {}", status_code, String::from_utf8_lossy(content.as_bytes()))),
                    }
                } else {
                    // Fallback if ErrorReason cannot be parsed
                    Error::Provider {
                        code: format!("HTTP_{}", status_code),
                        message: format!("MTN Collection API error (HTTP {}): {}", status_code, String::from_utf8_lossy(content.as_bytes())),
                    }
                }
            }
            psc_mtn_collection::apis::Error::Reqwest(e) => Error::Internal(format!("MTN Collection API Reqwest error: {}", e)),
            psc_mtn_collection::apis::Error::Serde(e) => Error::Internal(format!("MTN Collection API Serde error: {}", e)),
            psc_mtn_collection::apis::Error::Io(e) => Error::Internal(format!("MTN Collection API IO error: {}", e)),
        }
    }

    /// Helper to map MTN Disbursement API errors to our unified Error type.
    fn map_mtn_disbursement_error<T>(e: psc_mtn_disbursement::apis::Error<T>) -> Error {
        match e {
            psc_mtn_disbursement::apis::Error::ResponseError(response_error) => {
                let status_code = response_error.status.as_u16();
                let content = response_error.content;

                if let Ok(error_reason) = serde_json::from_slice::<MtnErrorReason>(content.as_bytes()) {
                    Error::Provider {
                        code: error_reason.code.unwrap_or_else(|| "UNKNOWN_MTN_DISBURSEMENT_ERROR_CODE".to_string()),
                        message: error_reason.message.unwrap_or_else(|| format!("MTN Disbursement API error (HTTP {}): {}", status_code, String::from_utf8_lossy(content.as_bytes()))),
                    }
                } else {
                    Error::Provider {
                        code: format!("HTTP_{}", status_code),
                        message: format!("MTN Disbursement API error (HTTP {}): {}", status_code, String::from_utf8_lossy(content.as_bytes())),
                    }
                }
            }
            _ => Error::Internal(format!("MTN Disbursement API error: {}", e)),
        }
    }

    /// Helper to map MTN Remittance API errors to our unified Error type.
    fn map_mtn_remittance_error<T>(e: psc_mtn_remittance::apis::Error<T>) -> Error {
        match e {
            psc_mtn_remittance::apis::Error::ResponseError(response_error) => {
                let status_code = response_error.status.as_u16();
                let content = response_error.content;

                if let Ok(error_reason) = serde_json::from_slice::<MtnErrorReason>(content.as_bytes()) {
                    Error::Provider {
                        code: error_reason.code.unwrap_or_else(|| "UNKNOWN_MTN_REMITTANCE_ERROR_CODE".to_string()),
                        message: error_reason.message.unwrap_or_else(|| format!("MTN Remittance API error (HTTP {}): {}", status_code, String::from_utf8_lossy(content.as_bytes()))),
                    }
                } else {
                    Error::Provider {
                        code: format!("HTTP_{}", status_code),
                        message: format!("MTN Remittance API error (HTTP {}): {}", status_code, String::from_utf8_lossy(content.as_bytes())),
                    }
                }
            }
            _ => Error::Internal(format!("MTN Remittance API error: {}", e)),
        }
    }

    /// Helper to map MTN Sandbox Provisioning API errors to our unified Error type.
    fn map_mtn_sandbox_provisioning_error<T>(e: psc_mtn_sandbox_provisioning::apis::Error<T>) -> Error {
        match e {
            psc_mtn_sandbox_provisioning::apis::Error::ResponseError(response_error) => {
                let status_code = response_error.status.as_u16();
                let content = response_error.content;

                if let Ok(error_reason) = serde_json::from_slice::<MtnErrorReason>(content.as_bytes()) {
                    Error::Provider {
                        code: error_reason.code.unwrap_or_else(|| "UNKNOWN_MTN_SANDBOX_PROVISIONING_ERROR_CODE".to_string()),
                        message: error_reason.message.unwrap_or_else(|| format!("MTN Sandbox Provisioning API error (HTTP {}): {}", status_code, String::from_utf8_lossy(content.as_bytes()))),
                    }
                } else {
                    Error::Provider {
                        code: format!("HTTP_{}", status_code),
                        message: format!("MTN Sandbox Provisioning API error (HTTP {}): {}", status_code, String::from_utf8_lossy(content.as_bytes())),
                    }
                }
            }
            _ => Error::Internal(format!("MTN Sandbox Provisioning API error: {}", e)),
        }
    }
}

// Struct to parse MTN's error response body
#[derive(Debug, Deserialize)]
struct MtnErrorReason {
    code: Option<String>,
    message: Option<String>,
}

#[async_trait]
impl Provider for MtnSandboxAdapter {
    async fn deposit(&self, _ctx: &Ctx, req: CreatePaymentRequest) -> Result<Payment> {
        // Map unified request to MTN RequestToPay
        let reference_id = if req.idempotency_key.is_empty() {
            cuid2()
        } else {
            req.idempotency_key.clone()
        };
        let (amount_minor, currency_code) = match &req.amount {
            Some(m) => (m.amount_minor_units, m.currency_code.clone()),
            None => (0, "XAF".to_string()),
        };
        let payer_msisdn = req
            .payer_id
            .as_ref()
            .map(|i| i.value.clone())
            .unwrap_or_else(|| "unknown".to_string());

        // Convert minor units to decimal string for MTN API (assume 2 dp)
        let amount_str = format!("{:.2}", (amount_minor as f64) / 100.0);

        // Map to MTN model
        let mtn_request_to_pay = psc_mtn_collection::models::RequestToPay {
            amount: Some(amount_str.clone()),
            currency: Some(currency_code.clone()),
            external_id: Some(reference_id.clone()),
            payer: Some(Box::new(psc_mtn_collection::models::Party { party_id_type: Some(psc_mtn_collection::models::party::PartyIdType::Msisdn), party_id: Some(payer_msisdn.clone()) })),
            payer_message: None,
            payee_note: Some("Payment collection".to_string()),
        };

        let x_target_environment = Some(self.config.target_environment.clone());
        let authorization = Some(format!("Bearer {}", self.config.api_key)); // Assuming API key is directly the bearer token
        let x_callback_url: Option<&str> = None;

        let result = psc_mtn_collection::apis::default_api::requestto_pay(
            &self.collection_cfg,
            authorization.as_deref().unwrap_or(""),
            &reference_id,
            x_target_environment.as_deref().unwrap_or("sandbox"),
            x_callback_url.as_deref(),
            Some(mtn_request_to_pay),
        )
        .await;

        match result {
            Ok(_) => {
                // Return PENDING; webhook updates later
                let payment = Payment {
                    id: Some(Id { value: cuid2() }),
                    amount: Some(Money { amount_minor_units: amount_minor, currency_code: currency_code.clone() }),
                    status: PaymentStatus::Pending as i32,
                    created_at: Some(Timestamp { value: Some(prost_types::Timestamp { seconds: time::OffsetDateTime::now_utc().unix_timestamp(), nanos: 0 }) }),
                    updated_at: Some(Timestamp { value: Some(prost_types::Timestamp { seconds: time::OffsetDateTime::now_utc().unix_timestamp(), nanos: 0 }) }),
                    metadata: Default::default(),
                    reference: reference_id.clone(),
                };

                // Publish event to NATS
                let event_payload = serde_json::json!({
                    "transaction_type": "deposit",
                    "reference_id": reference_id,
                    "status": "pending",
                    "provider": "MTN_SANDBOX",
                    "payer": payer_msisdn,
                    "amount": amount_str,
                    "currency": currency_code,
                });
                self.nats_client.publish("payments.status.update", event_payload.to_string().into_bytes()).await
                    .map_err(|e| Error::Internal(format!("Failed to publish NATS event: {}", e)))?;

                Ok(payment)
            }
            Err(e) => {
                Err(Self::map_mtn_collection_error(e))
            }
        }
    }

    async fn withdraw(&self, _ctx: &Ctx, req: CreatePayoutRequest) -> Result<Payout> {
        let reference_id = if req.idempotency_key.is_empty() {
            cuid2()
        } else {
            req.idempotency_key.clone()
        };
        let (amount_minor, currency_code) = match &req.amount {
            Some(m) => (m.amount_minor_units, m.currency_code.clone()),
            None => (0, "XAF".to_string()),
        };
        let recipient_msisdn = req
            .recipient_id
            .as_ref()
            .map(|i| i.value.clone())
            .unwrap_or_else(|| "unknown".to_string());

        let amount_str = format!("{:.2}", (amount_minor as f64) / 100.0);

        let mtn_disbursement_request = psc_mtn_disbursement::models::Transfer {
            amount: Some(amount_str.clone()),
            currency: Some(currency_code.clone()),
            external_id: Some(reference_id.clone()),
            payee: Some(Box::new(psc_mtn_disbursement::models::Party { party_id_type: Some(psc_mtn_disbursement::models::party::PartyIdType::Msisdn), party_id: Some(recipient_msisdn.clone()) })),
            payer_message: None,
            payee_note: Some("Payment disbursement".to_string()),
        };

        let x_target_environment = Some(self.config.target_environment.clone());
        let authorization = Some(format!("Bearer {}", self.config.api_key));
        let x_callback_url: Option<&str> = None;

        let result = psc_mtn_disbursement::apis::default_api::transfer(
            &self.disbursement_cfg,
            authorization.as_deref().unwrap_or(""),
            &reference_id,
            x_target_environment.as_deref().unwrap_or("sandbox"),
            x_callback_url.as_deref(),
            Some(mtn_disbursement_request),
        )
        .await;

        match result {
            Ok(_) => {
                let payout = Payout {
                    id: Some(Id { value: cuid2() }),
                    amount: Some(Money { amount_minor_units: amount_minor, currency_code: currency_code.clone() }),
                    status: PayoutStatus::Pending as i32,
                    created_at: Some(Timestamp { value: Some(prost_types::Timestamp { seconds: time::OffsetDateTime::now_utc().unix_timestamp(), nanos: 0 }) }),
                    updated_at: Some(Timestamp { value: Some(prost_types::Timestamp { seconds: time::OffsetDateTime::now_utc().unix_timestamp(), nanos: 0 }) }),
                    external_reference: reference_id.clone(),
                    metadata: Default::default(),
                };

                // Publish event to NATS
                let event_payload = serde_json::json!({
                    "transaction_type": "withdraw",
                    "reference_id": reference_id,
                    "status": "pending",
                    "provider": "MTN_SANDBOX",
                    "recipient": recipient_msisdn,
                    "amount": amount_str,
                    "currency": currency_code,
                });
                self.nats_client
                    .publish("payouts.status.update", event_payload.to_string().into_bytes())
                    .await
                    .map_err(|e| Error::Internal(format!("Failed to publish NATS event: {}", e)))?;

                Ok(payout)
            }
            Err(e) => Err(Self::map_mtn_disbursement_error(e)),
        }
    }

    async fn refund(&self, _ctx: &Ctx, req: PostJournalRequest) -> Result<JournalEntry> {
        let reference_id = if req.idempotency_key.is_empty() {
            cuid2()
        } else {
            req.idempotency_key.clone()
        };

        let first = req.entries.get(0);
        let (amount_minor, currency_code, account) = match first.and_then(|e| e.amount.as_ref()) {
            Some(m) => (
                m.amount_minor_units,
                m.currency_code.clone(),
                first.unwrap().account.clone(),
            ),
            None => (0, "XAF".to_string(), first.map(|e| e.account.clone()).unwrap_or_default()),
        };

        let amount_str = format!("{:.2}", (amount_minor as f64) / 100.0);

        let mtn_remittance_request = psc_mtn_remittance::models::Transfer {
            amount: Some(amount_str.clone()),
            currency: Some(currency_code.clone()),
            external_id: Some(reference_id.clone()),
            payee: Some(Box::new(psc_mtn_remittance::models::Party { party_id_type: Some(psc_mtn_remittance::models::party::PartyIdType::Msisdn), party_id: Some(account.clone()) })),
            payer_message: None,
            payee_note: Some("Payment refund/remittance".to_string()),
        };

        let x_target_environment = Some(self.config.target_environment.clone());
        let authorization = Some(format!("Bearer {}", self.config.api_key));
        let x_callback_url: Option<&str> = None;

        let result = psc_mtn_remittance::apis::default_api::transfer(
            &self.remittance_cfg,
            authorization.as_deref().unwrap_or(""),
            &reference_id,
            x_target_environment.as_deref().unwrap_or("sandbox"),
            x_callback_url.as_deref(),
            Some(mtn_remittance_request),
        )
        .await;

        match result {
            Ok(_) => Ok(JournalEntry {
                id: Some(Id { value: cuid2() }),
                amount: Some(Money { amount_minor_units: amount_minor, currency_code }),
                r#type: first.map(|e| e.r#type).unwrap_or_default(),
                account,
                posted_at: Some(Timestamp { value: Some(prost_types::Timestamp { seconds: time::OffsetDateTime::now_utc().unix_timestamp(), nanos: 0 }) }),
                reference: reference_id,
                metadata: first.map(|e| e.metadata.clone()).unwrap_or_default(),
            }),
            Err(e) => Err(Self::map_mtn_remittance_error(e)),
        }
    }

    async fn query(&self, _ctx: &Ctx, req: GetBalanceRequest) -> Result<Balance> {
        let account_id = req
            .account_id
            .as_ref()
            .map(|i| i.value.clone())
            .unwrap_or_else(|| "unknown".to_string());

        let x_target_environment = Some(self.config.target_environment.clone());
        let authorization = Some(format!("Bearer {}", self.config.api_key));

        let result = psc_mtn_collection::apis::default_api::get_account_balance(
            &self.collection_cfg,
            authorization.as_deref().unwrap_or(""),
            x_target_environment.as_deref().unwrap_or("sandbox"),
        )
        .await;

        match result {
            Ok(mtn_balance) => {
                let currency = mtn_balance
                    .currency
                    .clone()
                    .unwrap_or_else(|| "XAF".to_string());
                let available_minor = mtn_balance
                    .available_balance
                    .as_deref()
                    .map(|s| {
                        // parse decimal string assuming 2 fractional digits
                        let d = rust_decimal::Decimal::from_str(s)
                            .unwrap_or(rust_decimal::Decimal::ZERO);
                        (d * rust_decimal::Decimal::from(100u64))
                            .round()
                            .to_i64()
                            .unwrap_or(0)
                    })
                    .unwrap_or(0);

                let money_available = Money { amount_minor_units: available_minor, currency_code: currency.clone() };
                let balance = Balance {
                    account_id: Some(Id { value: account_id }),
                    available: Some(money_available.clone()),
                    reserved: Some(Money { amount_minor_units: 0, currency_code: currency.clone() }),
                    ledger: Some(money_available),
                    as_of: Some(Timestamp { value: Some(prost_types::Timestamp { seconds: time::OffsetDateTime::now_utc().unix_timestamp(), nanos: 0 }) }),
                    metadata: Default::default(),
                };

                Ok(balance)
            }
            Err(e) => Err(Self::map_mtn_collection_error(e)),
        }
    }

    async fn verify_webhook(
        &self,
        _ctx: &Ctx,
        payload: &[u8],
        signature_header: Option<&str>,
    ) -> Result<bool> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let expected_signature = match signature_header {
            Some(s) => s.to_string(),
            None => return Ok(false), // No signature header, cannot verify
        };

        let key = self.config.webhook_secret.as_bytes();
        let mut mac = HmacSha256::new_from_slice(key)
            .map_err(|_| Error::Internal("Failed to create HMAC key".to_string()))?;

        mac.update(payload);
        let result = mac.finalize();
        let signature_bytes = result.into_bytes();

        let actual_signature = hex::encode(signature_bytes);

        // Simple comparison for now. In a real scenario, you might need to parse the header
        // (e.g., "sha256=<signature>") and handle timing attacks.
        Ok(actual_signature == expected_signature)
    }
}
