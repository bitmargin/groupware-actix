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

use crate::config::db_database;
use crate::database::{DbConn, DbPool};

#[derive(Debug, Deserialize)]
pub struct FindCompaniesParams {
    pub search: Option<String>,
    pub sort_by: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteCompanyParams {
    pub mode: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Company {
    pub name: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub modified_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Clone for Company {
    fn clone(&self) -> Company {
        Company {
            name: self.name.clone(),
            since: self.since.clone(),
            created_at: self.created_at.clone(),
            modified_at: self.modified_at.clone(),
            deleted_at: self.deleted_at.clone(),
        }
    }
}

// Implementation for Company struct, functions for read/write/update and delete todo from database
impl Company {
    pub fn find(params: &web::Query<FindCompaniesParams>, pool: &DbPool) -> Result<Vec<Company>, &'static str> {
        let conn: DbConn = pool.get().unwrap();
        let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
        let mut terms = vec!["FOR c IN companies"];
        let mut vars: HashMap<&str, Value> = HashMap::new();
        let search = params.search.as_ref();
        if search.is_some() {
            terms.push("FILTER CONTAINS(c.name, @@search)");
            vars.insert("@search", to_value(search).unwrap());
        }
        let sort_by = params.sort_by.as_ref();
        if sort_by.is_some() {
            terms.push("SORT c.@@sort_by ASC");
            vars.insert("@sort_by", to_value(sort_by).unwrap());
        }
        let limit = params.limit.as_ref();
        if limit.is_some() {
            terms.push("LIMIT 0, @@limit");
            vars.insert("@limit", to_value(limit).unwrap());
        }
        terms.push("RETURN c");
        let q = terms.join(" ");
        let aql = AqlQuery::builder()
            .query(&q)
            .bind_vars(vars)
            .build();
        let records: Vec<Company> = db.aql_query(aql).unwrap();
        Ok(records)
    }

    pub fn show(key: &web::Path<String>, pool: &DbPool) -> Result<Company, &'static str> {
        let conn: DbConn = pool.get().unwrap();
        let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
        let collection: Collection<ReqwestClient> = db.collection("companies").unwrap();
        let res: Document<Company> = collection.document(key.as_ref()).unwrap();
        let record: Company = res.document;
        Ok(record)
    }

    pub fn create(params: web::Form<Company>, pool: &DbPool) -> Result<Company, &'static str> {
        let conn: DbConn = pool.get().unwrap();
        let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
        let collection: Collection<ReqwestClient> = db.collection("companies").unwrap();
        let now = Utc::now();
        let data = Company {
            name: params.name.clone(),
            since: params.since,
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

    pub fn update(key: &web::Path<String>, params: web::Form<Company>, pool: &DbPool) -> Result<Company, &'static str> {
        let conn: DbConn = pool.get().unwrap();
        let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
        let collection: Collection<ReqwestClient> = db.collection("companies").unwrap();
        let obj: Value = json!({
            "modified_at": Utc::now(),
        });
        let text: String = to_string(&obj).unwrap();
        let mut data: Company = from_str::<Company>(&text).unwrap();
        if params.name.is_some() {
            data.name = params.name.clone();
        }
        if params.since.is_some() {
            data.since = params.since.clone();
        }
        let options: UpdateOptions = UpdateOptions::builder()
            .return_new(true)
            .return_old(true)
            .build();
        let res: DocumentResponse<Document<Company>> = collection.update_document(key, Document::new(data), options).unwrap();
        let record: &Company = res.new_doc().unwrap();
        Ok(record.clone())
    }

    pub fn erase(key: &web::Path<String>, pool: &DbPool) -> Result<Company, &'static str> {
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

    pub fn trash(key: &web::Path<String>, pool: &DbPool) -> Result<Company, &'static str> {
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

    pub fn restore(key: &web::Path<String>, pool: &DbPool) -> Result<Company, &'static str> {
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
