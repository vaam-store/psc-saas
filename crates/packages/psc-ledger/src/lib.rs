use psc_domain::Money;
use psc_error::Result;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;
use uuid::Uuid; // Use Uuid temporarily

mod service;

pub mod pb {
    pub mod psc {
        pub mod common {
            pub mod v1 {
                tonic::include_proto!("psc.common.v1");
            }
        }
        pub mod journal {
            pub mod v1 {
                tonic::include_proto!("psc.journal.v1");
            }
        }
    }
}

#[derive(Debug, FromRow, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Account {
    pub id: Uuid, // Changed from Cuid to Uuid
    pub name: String,
    #[sqlx(rename = "type")]
    pub account_type: String,
    pub currency: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, FromRow, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Journal {
    pub id: Uuid, // Changed from Cuid to Uuid
    pub description: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, FromRow, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct JournalEntry {
    pub id: Uuid,           // Changed from Cuid to Uuid
    pub journal_id: Uuid,   // Changed from Cuid to Uuid
    pub account_id: Uuid,   // Changed from Cuid to Uuid
    pub entry_type: String, // "DEBIT" or "CREDIT"
    pub amount_minor_units: i64,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

// Enum for entry type to ensure type safety
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EntryType {
    Debit,
    Credit,
}
pub struct LedgerRepository {
    pool: PgPool,
}

impl LedgerRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_account(
        &self,
        name: String,
        account_type: String,
        currency: String,
    ) -> Result<Account> {
        let account = sqlx::query_as!(
            Account,
            r#"
            INSERT INTO accounts (id, name, type, currency)
            VALUES ($1, $2, $3, $4)
            RETURNING id, name, type as "account_type", currency, created_at, updated_at
            "#,
            Uuid::new_v4(),
            name,
            account_type,
            currency
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(account)
    }

    pub async fn get_account_by_id(&self, id: Uuid) -> Result<Option<Account>> {
        let account = sqlx::query_as!(
            Account,
            r#"
            SELECT id, name, type as "account_type", currency, created_at, updated_at
            FROM accounts
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(account)
    }

    pub async fn get_account_by_name(&self, name: &str) -> Result<Option<Account>> {
        let account = sqlx::query_as!(
            Account,
            r#"
            SELECT id, name, type as "account_type", currency, created_at, updated_at
            FROM accounts
            WHERE name = $1
            "#,
            name
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(account)
    }
    pub async fn create_journal_with_entries(
        &self,
        description: Option<String>,
        entries: Vec<(Uuid, EntryType, i64)>, // (account_id, entry_type, amount_minor_units)
    ) -> Result<Journal> {
        // 1. Validate debit/credit invariant
        let mut total_debits: i64 = 0;
        let mut total_credits: i64 = 0;

        for (_, entry_type, amount) in &entries {
            match entry_type {
                EntryType::Debit => total_debits += amount,
                EntryType::Credit => total_credits += amount,
            }
        }

        if total_debits != total_credits {
            return Err(psc_error::Error::BadRequest(
                "Debit and credit amounts do not balance for journal entry".to_string(),
            ));
        }

        let mut tx = self.pool.begin().await?;

        // 2. Create the journal
        let journal = sqlx::query_as!(
            Journal,
            r#"
            INSERT INTO journals (id, description)
            VALUES ($1, $2)
            RETURNING id, description, created_at, updated_at
            "#,
            Uuid::new_v4(),
            description
        )
        .fetch_one(&mut *tx)
        .await?;

        // 3. Create journal entries
        for (account_id, entry_type, amount) in entries {
            sqlx::query!(
                r#"
                INSERT INTO journal_entries (id, journal_id, account_id, entry_type, amount_minor_units)
                VALUES ($1, $2, $3, $4, $5)
                "#,
                Uuid::new_v4(),
                journal.id,
                account_id,
                entry_type.to_string(),
                amount
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(journal)
    }
}
