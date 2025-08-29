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

        let entries_to_create: Vec<(Uuid, EntryType, i64)> = request
            .entries
            .into_iter()
            .filter_map(|entry| {
                let account_id_uuid = match Uuid::parse_str(&entry.account) {
                    Ok(uuid) => uuid,
                    Err(_) => return None, // Or handle error appropriately
                };
                let entry_type = match ProtoEntryType::try_from(entry.r#type) {
                    Ok(ProtoEntryType::Debit) => EntryType::Debit,
                    Ok(ProtoEntryType::Credit) => EntryType::Credit,
                    _ => return None, // Or handle unknown/unspecified entry type
                };
                Some((
                    account_id_uuid,
                    entry_type,
                    entry.amount.unwrap().amount_minor_units,
                )) // Corrected field name
            })
            .collect();

        // Convert the psc_error::Error to tonic::Status
        let journal = self
            .repository
            .create_journal_with_entries(request.narrative.into(), entries_to_create) // Converted String to Option<String>
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
