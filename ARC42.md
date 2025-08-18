# arc42.md

# 1. Introduction and Goals  
The platform is a **Cameroon-only digital payments solution** that combines **escrow-based flows (PayPal-like)** and **simple charge flows (Stripe-like)**.  
It solves the fragmentation of mobile money ecosystems (MTN, Orange, Camtel) by providing a **unified API**, a **developer-friendly integration model**, and a **trust layer** (escrow).  

**Key goals:**  
- Enable customers to pay via MTN/Orange/Camtel.  
- Allow merchants to cash out on their preferred provider.  
- Provide both escrow (secure, trust-building) and charge (instant, convenient) flows.  
- Ensure correctness with a double-entry ledger and reconciliation.  
- Provide visibility and control via dashboards for merchants, customers, and treasury operators.  

**Non-goals (for MVP):**  
- No support for card payments or international rails.  
- No multi-currency (only XAF supported initially).  
- No complex dispute arbitration workflow in MVP.  

---

# 2. Constraints  
- **Regulatory**: Must comply with Cameroonian financial regulations, KYC/AML rules, and data residency requirements.  
- **Technical**:  
  - Language: Rust (axum + tonic).  
  - Infrastructure: Kubernetes + Knative, Postgres, Redis, NATS/Kafka, Vault.  
  - Identity: Keycloak for organisations and users.  
- **Operational**:  
  - Provider APIs (MTN, Orange, Camtel) are often unstable and under-documented.  
  - Float liquidity must be maintained across multiple wallets.  

---

# 3. Context and Scope  
**Users**:  
- Customers: initiate payments.  
- Merchants: receive payments, withdraw funds.  
- Org Admins: manage organisation-level reporting and controls.  
- Treasury Operators: ensure liquidity across provider accounts.  

**External systems**:  
- MTN/Orange/Camtel APIs for collection, disbursement, refund, and transaction queries.  
- Bank APIs for wallet top-ups.  
- Keycloak for authentication and authorisation.  

**System Scope**:  
- Provide a single API for charges, escrow, refunds, payouts.  
- Maintain an authoritative ledger.  
- Handle liquidity and reconciliation.  

---

# 4. Solution Strategy  
- Use **double-entry ledger** as the source of truth for all money movement.  
- Abstract away provider differences via a **provider gateway**.  
- Use **escrow flows** for high-trust requirements and **charge flows** for instant settlement.  
- Manage liquidity via **treasury service** (monitoring, clearing, top-ups).  
- Ensure correctness via **reconciliation service**.  
- Scale reliably with Kubernetes and provide **observability** out of the box.  

---

# 5. Building Block View  

**Core Services**:  
- **Ledger Service**: Maintains journals and balances, enforces invariants.  
- **Payments Service**: Escrow and charge-based flows (capture, release, refund).  
- **Payouts Service**: Merchant withdrawals, fees, provider disbursement.  
- **Treasury Service**: Liquidity monitoring, clearing, top-ups.  
- **Provider Gateway**: Unified integration with MTN, Orange, Camtel; webhook verification.  
- **Reconciliation Service**: Imports provider statements, matches, posts adjustments.  
- **BFF/API**: Exposes REST/OpenAPI for merchants, customers, and NextJS console.  

**Shared Libraries**:  
- Protos, types, errors, config, observability, auth, idempotency, fees, provider traits, testkit.  

---

# 6. Runtime View  

**Escrow Flow:**  
1. Customer pays via MTN → Provider Gateway collects money.  
2. Payments Service posts DR MTN Float / CR Escrow Payable.  
3. Merchant fulfills order.  
4. Payments Service posts DR Escrow Payable / CR Merchant Payable.  
5. Merchant withdraws → Payouts Service posts DR Merchant Payable / CR Provider Float → Provider Gateway executes cash-out.  

**Charge Flow:**  
1. Customer pays via MTN → Provider Gateway collects money.  
2. Payments Service posts DR MTN Float / CR Merchant Payable.  
3. Merchant automatically receives funds (payout triggered).  

**Refund Flow:**  
1. Refund requested.  
2. Payments Service posts DR Merchant Payable (or Escrow Payable) / CR Provider Float.  
3. Provider Gateway issues refund to customer.  

**Treasury Flow:**  
1. Treasury monitors floats.  
2. Orange float low, MTN float high → Treasury posts clearing move.  
3. Bank API executes actual transfer between provider accounts.  

---

# 7. Deployment View  
- **Kubernetes (K8s)**: Each server is a deployment with autoscaling.  
- **Knative**: Scale-to-zero for provider-gateway adapters (bursty workloads).  
- **Postgres**: Shared DB for ledger and services; with migrations per bounded context.  
- **Redis**: For idempotency keys and lightweight queues.  
- **NATS/Kafka**: Event bus for asynchronous workflows.  
- **Vault/KMS**: Store provider secrets and encryption keys.  
- **Observability Stack**: OTEL collector + Grafana + Loki.  

---

# 8. Cross-cutting Concepts  
- **Idempotency**: All mutating endpoints accept `idempotency-key`; Redis used to store responses.  
- **Authentication**: Keycloak JWT; roles (customer, merchant, org-admin, platform).  
- **Authorisation**: Org-based scoping enforced in API and services.  
- **Error Handling**: Unified taxonomy; retryable classification; exponential backoff with jitter.  
- **Data Integrity**: Debit = Credit invariant; single-currency journals.  
- **Auditability**: Every journal and provider transaction stored with traceable reference.  
- **Observability**: Traces for every request; metrics on success/failure rates; logs with correlation IDs.  

---

# 9. Architecture Decisions  
- **ADR-001**: All money movement must go through double-entry ledger.  
- **ADR-002**: Escrow and charge are two different payment event types but share the same posting engine.  
- **ADR-003**: Provider integrations abstracted in provider-gateway to isolate business services from external API differences.  
- **ADR-004**: Use Postgres as system of record, Redis for fast idempotency.  
- **ADR-005**: Use gRPC internally for service-to-service contracts; HTTP/REST only at edge.  
- **ADR-006**: Use outbox/inbox pattern for reliable event delivery.  

---

# 10. Risks and Technical Debt  
- **Provider API instability**: Mitigation → build provider-mock for local dev; retry logic with backoff.  
- **Liquidity imbalance across providers**: Mitigation → Treasury clearing moves + top-up policies.  
- **Recon complexity**: Mitigation → Automate imports, fuzzy matching, operator tools.  
- **Scope creep (escrow + charge + payouts at once)**: Mitigation → Focus MVP on `CreateCharge` + `CreateEscrow` + single provider integration (MTN).  
- **Operational overhead**: Mitigation → Observability, automation, standard SRE practices.  

---

# 11. Quality Scenarios  
- **Performance**: Handle 200 RPS sustained pay-ins, with <300ms p99 latency under warm cache.  
- **Reliability**: Survive provider outages gracefully; retries do not duplicate journals.  
- **Auditability**: Ability to trace from customer payment ID → ledger journal → provider transaction ID.  
- **Security**: Protect user balances; enforce JWT + org scopes.  
- **Scalability**: Autoscale provider-gateway to handle spikes in pay-ins/payouts.  

---

# 12. Glossary  
- **Escrow**: Funds held by the platform until merchant fulfills their duty.  
- **Charge**: Immediate settlement to merchant without escrow.  
- **Float**: Money held in MTN/Orange/Camtel wallets that funds payouts.  
- **Clearing**: Internal balancing process between provider floats.  
- **Journal**: Ledger record of a business event (debits and credits).  
- **Principal**: An actor (customer, merchant, platform, treasury) with an account in the system.  
