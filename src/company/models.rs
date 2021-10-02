use actix_web::web;
use anyhow::Result;
use arangors::{
    document::{
        options::{InsertOptions, RemoveOptions, UpdateOptions},
        response::DocumentResponse,
    },
    AqlQuery, Collection, Database, Document,
};
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, json, to_string, to_value, Value};
use std::collections::HashMap;
use uclient::reqwest::ReqwestClient;
use validator::{Validate, ValidationError, ValidationErrors};

use crate::config::db_database;
use crate::database::{DbConn, DbPool};

#[derive(Clone, Debug, Validate, Deserialize)]
pub struct FindCompaniesParams {
    pub search: Option<String>,
    #[validate(custom = "validate_sort_by")]
    pub sort_by: Option<String>,
    #[validate(range(min = 1, max = 100))]
    pub limit: Option<u32>,
}

fn validate_sort_by(sort_by: &str) -> Result<(), ValidationError> {
    if sort_by != "name" && sort_by != "since" {
        return Err(ValidationError::new("Wrong sort_by"));
    }
    Ok(())
}

impl FindCompaniesParams {
    pub fn check_valid(&self) -> Result<(), ValidationErrors> { // public version of validate
        self.validate()
    }
}

#[derive(Debug, Validate, Deserialize)]
pub struct DeleteCompanyParams {
    #[validate(custom = "validate_mode")]
    pub mode: String,
}

fn validate_mode(mode: &str) -> Result<(), ValidationError> {
    if mode != "erase" && mode != "trash" && mode != "restore" {
        return Err(ValidationError::new("Wrong mode"));
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Company {
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub since: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub created_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub modified_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub deleted_at: Option<DateTime<Utc>>,
}

// Implementation for Company struct, functions for read/write/update and delete todo from database
impl Company {
    pub fn find(params: FindCompaniesParams, pool: &DbPool) -> Result<Vec<Company>, ValidationErrors> {
        let conn: DbConn = pool.get().unwrap();
        let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
        let mut terms = vec!["FOR c IN companies"];
        let mut vars: HashMap<&str, Value> = HashMap::new();
        if params.search.is_some() {
            let search: String = params.search.unwrap().trim().to_string();
            if !search.is_empty() {
                terms.push("FILTER CONTAINS(c.name, @@search)");
                vars.insert("@search", to_value(search).unwrap());
            }
        }
        if params.sort_by.is_some() {
            let sort_by: String = params.sort_by.unwrap();
            terms.push("SORT c.@@sort_by ASC");
            vars.insert("@sort_by", to_value(sort_by).unwrap());
        }
        if params.limit.is_some() {
            let limit: u32 = params.limit.unwrap();
            terms.push("LIMIT 0, @@limit");
            vars.insert("@limit", to_value(limit).unwrap());
        }
        terms.push("RETURN c");
        let q = terms.join(" ");
        let aql = AqlQuery::builder()
            .query(&q)
            .bind_vars(vars)
            .build();
        let records: Vec<Company> = db.aql_query(aql).expect("Query failed");
        Ok(records)
    }

    pub fn show(key: &String, pool: &DbPool) -> Result<Company, &'static str> {
        let conn: DbConn = pool.get().unwrap();
        let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
        let collection: Collection<ReqwestClient> = db.collection("companies").unwrap();
        let res: Document<Company> = collection.document(key.as_ref()).unwrap();
        let record: Company = res.document;
        Ok(record)
    }

    pub fn create(payload: &web::Json<Company>, pool: &DbPool) -> Result<Company, &'static str> {
        let conn: DbConn = pool.get().unwrap();
        let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
        let collection: Collection<ReqwestClient> = db.collection("companies").unwrap();
        let now = Utc::now();
        let data = Company {
            name: payload.name.clone(),
            since: payload.since,
            created_at: Some(now),
            modified_at: Some(now),
            deleted_at: None,
        };
        let options: InsertOptions = InsertOptions::builder()
            .return_new(true)
            .build();
        let res: DocumentResponse<Document<Company>> = collection.create_document(Document::new(data), options).unwrap();
        let record: &Company = res.new_doc().unwrap();
        Ok(record.clone())
    }

    pub fn update(key: &String, payload: &web::Json<Company>, pool: &DbPool) -> Result<Company, &'static str> {
        let conn: DbConn = pool.get().unwrap();
        let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
        let collection: Collection<ReqwestClient> = db.collection("companies").unwrap();
        let obj: Value = json!({
            "modified_at": Utc::now(),
        });
        let text: String = to_string(&obj).unwrap();
        let mut data: Company = from_str::<Company>(&text).unwrap();
        if payload.name.is_some() {
            data.name = payload.name.clone();
        }
        if payload.since.is_some() {
            data.since = payload.since.clone();
        }
        let options: UpdateOptions = UpdateOptions::builder()
            .return_new(true)
            .return_old(true)
            .build();
        let res: DocumentResponse<Document<Company>> = collection.update_document(key, Document::new(data), options).unwrap();
        let record: &Company = res.new_doc().unwrap();
        Ok(record.clone())
    }

    pub fn erase(key: &String, pool: &DbPool) -> Result<Company, &'static str> {
        let conn: DbConn = pool.get().unwrap();
        let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
        let collection: Collection<ReqwestClient> = db.collection("companies").unwrap();
        let options: RemoveOptions = RemoveOptions::builder()
            .return_old(true)
            .build();
        let res: DocumentResponse<Document<Company>> = collection.remove_document(key.as_ref(), options, None).unwrap();
        let record: &Company = res.old_doc().unwrap();
        Ok(record.clone())
    }

    pub fn trash(key: &String, pool: &DbPool) -> Result<Company, &'static str> {
        let conn: DbConn = pool.get().unwrap();
        let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
        let collection: Collection<ReqwestClient> = db.collection("companies").unwrap();
        let obj = json!({
            "deleted_at": Utc::now(),
        });
        let text = to_string(&obj).unwrap();
        let data: Company = from_str::<Company>(&text).unwrap();
        let options: UpdateOptions = UpdateOptions::builder()
            .return_new(true)
            .return_old(true)
            .build();
        let res: DocumentResponse<Document<Company>> = collection.update_document(key, Document::new(data), options).unwrap();
        let record: &Company = res.new_doc().unwrap();
        Ok(record.clone())
    }

    pub fn restore(key: &String, pool: &DbPool) -> Result<Company, &'static str> {
        let conn: DbConn = pool.get().unwrap();
        let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
        let collection: Collection<ReqwestClient> = db.collection("companies").unwrap();
        let data: Company = from_str::<Company>("{\"deleted_at\":null}").unwrap();
        let options: UpdateOptions = UpdateOptions::builder()
            .return_new(true)
            .return_old(true)
            .keep_null(false)
            .build();
        let res: DocumentResponse<Document<Company>> = collection.update_document(key, Document::new(data), options).unwrap();
        let record: &Company = res.new_doc().unwrap();
        Ok(record.clone())
    }
}
