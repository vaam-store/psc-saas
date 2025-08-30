use crate::EntryType;
use crate::LedgerRepository;
use psc_error::Error;
use sqlx::PgPool;
use tonic::{Request, Response, Status};
use uuid::Uuid;

// Generated Protobuf files (now imported from crate::pb)
use crate::pb::psc::common::v1::{Id as ProtoId, Money as ProtoMoney}; // Import Money and Id
use crate::pb::psc::journal::v1::{
    EntryType as ProtoEntryType, // Import Proto EntryType
    GetJournalEntryRequest,
    GetJournalEntryResponse,
    ListJournalEntriesRequest,
    ListJournalEntriesResponse,
    PostJournalRequest,
    PostJournalResponse,
    journal_service_server::JournalService as JournalServiceTrait,
};

pub struct JournalService {
    repository: LedgerRepository,
}

impl JournalService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            repository: LedgerRepository::new(pool),
        }
    }
}

#[tonic::async_trait]
impl JournalServiceTrait for JournalService {
    async fn post_journal(
        &self,
        request: Request<PostJournalRequest>,
    ) -> Result<Response<PostJournalResponse>, Status> {
        let request = request.into_inner();

        // Prepare optional description from narrative
        let description = if request.narrative.trim().is_empty() {
            None
        } else {
            Some(request.narrative.clone())
        };

        // Validate and transform entries; fail fast on first invalid entry
        let entries_to_create: Vec<(Uuid, EntryType, i64)> = request
            .entries
            .into_iter()
            .enumerate()
            .map(|(idx, entry)| {
                let account_id_uuid = Uuid::parse_str(&entry.account).map_err(|_| {
                    Status::invalid_argument(format!(
                        "entries[{}].account is not a valid UUID",
                        idx
                    ))
                })?;

                let entry_type = match ProtoEntryType::try_from(entry.r#type) {
                    Ok(ProtoEntryType::Debit) => EntryType::Debit,
                    Ok(ProtoEntryType::Credit) => EntryType::Credit,
                    Ok(_) | Err(_) => {
                        return Err(Status::invalid_argument(format!(
                            "entries[{}].type is unspecified or unknown",
                            idx
                        )))
                    }
                };

                let amount_minor_units = entry
                    .amount
                    .as_ref()
                    .map(|m| m.amount_minor_units)
                    .ok_or_else(|| {
                        Status::invalid_argument(format!(
                            "entries[{}].amount is required",
                            idx
                        ))
                    })?;

                Ok::<(Uuid, EntryType, i64), Status>((
                    account_id_uuid,
                    entry_type,
                    amount_minor_units,
                ))
            })
            .collect::<Result<Vec<_>, Status>>()?;

        // Convert the psc_error::Error to tonic::Status
        let _journal = self
            .repository
            .create_journal_with_entries(description, entries_to_create)
            .await
            .map_err(|e| match e {
                Error::BadRequest(msg) => Status::invalid_argument(msg),
                _ => Status::internal(e.to_string()),
            })?;

        let response = PostJournalResponse {
            posted_entries: vec![], // TODO: Populate with actual posted entries
        };

        Ok(Response::new(response))
    }

    async fn get_journal_entry(
        &self,
        _request: Request<GetJournalEntryRequest>,
    ) -> Result<Response<GetJournalEntryResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn list_journal_entries(
        &self,
        _request: Request<ListJournalEntriesRequest>,
    ) -> Result<Response<ListJournalEntriesResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }
}
