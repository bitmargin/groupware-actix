use actix_web::web;
use arangors::{
    connection::ReqwestClient,
    document::{
        options::{InsertOptions, RemoveOptions, UpdateOptions},
        response::DocumentResponse,
    },
    AqlQuery, Collection, Database, Document,
};
use chrono::prelude::*;
use serde_json::{from_str, json, to_string, to_value, Value};
use std::collections::HashMap;
use validator::ValidationErrors;

use crate::config::db_database;
use crate::database::DbPool;
use crate::company::{
    Company,
    DeleteCompanyParams,
    FindCompaniesParams,
};

pub async fn find_companies(
    params: FindCompaniesParams,
    pool: &DbPool,
) -> Result<Vec<Company>, ValidationErrors> {
    let client = pool.get().await.unwrap();
    let db = client.db(&db_database()).await.unwrap();

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
    let records: Vec<Company> = db.aql_query(aql).await.unwrap();
    Ok(records)
}

pub async fn show_company(
    key: &String,
    pool: &DbPool,
) -> Result<Company, &'static str> {
    let client = pool.get().await.unwrap();
    let db = client.db(&db_database()).await.unwrap();

    let collection: Collection<ReqwestClient> = db.collection("companies").await.unwrap();
    let res: Document<Company> = collection.document(key.as_ref()).await.unwrap();
    let record: Company = res.document;
    Ok(record)
}

pub async fn create_company(
    payload: &web::Json<Company>,
    pool: &DbPool,
) -> Result<Company, &'static str> {
    let client = pool.get().await.unwrap();
    let db = client.db(&db_database()).await.unwrap();

    let collection: Collection<ReqwestClient> = db.collection("companies").await.unwrap();
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

    let res: DocumentResponse<Document<Company>> = collection.create_document(Document::new(data), options).await.unwrap();
    let record: &Company = res.new_doc().unwrap();
    Ok(record.clone())
}

pub async fn update_company(
    key: &String,
    payload: &web::Json<Company>,
    pool: &DbPool,
) -> Result<Company, &'static str> {
    let client = pool.get().await.unwrap();
    let db = client.db(&db_database()).await.unwrap();

    let collection: Collection<ReqwestClient> = db.collection("companies").await.unwrap();
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

    let res: DocumentResponse<Document<Company>> = collection.update_document(key, Document::new(data), options).await.unwrap();
    let record: &Company = res.new_doc().unwrap();
    Ok(record.clone())
}

pub async fn erase_company(
    key: &String,
    pool: &DbPool,
) -> Result<Company, &'static str> {
    let client = pool.get().await.unwrap();
    let db = client.db(&db_database()).await.unwrap();

    let collection: Collection<ReqwestClient> = db.collection("companies").await.unwrap();
    let options: RemoveOptions = RemoveOptions::builder()
        .return_old(true)
        .build();

    let res: DocumentResponse<Document<Company>> = collection.remove_document(key.as_ref(), options, None).await.unwrap();
    let record: &Company = res.old_doc().unwrap();
    Ok(record.clone())
}

pub async fn trash_company(
    key: &String,
    pool: &DbPool,
) -> Result<Company, &'static str> {
    let client = pool.get().await.unwrap();
    let db = client.db(&db_database()).await.unwrap();

    let collection: Collection<ReqwestClient> = db.collection("companies").await.unwrap();
    let obj = json!({
        "deleted_at": Utc::now(),
    });
    let text = to_string(&obj).unwrap();
    let data: Company = from_str::<Company>(&text).unwrap();
    let options: UpdateOptions = UpdateOptions::builder()
        .return_new(true)
        .return_old(true)
        .build();

    let res: DocumentResponse<Document<Company>> = collection.update_document(key, Document::new(data), options).await.unwrap();
    let record: &Company = res.new_doc().unwrap();
    Ok(record.clone())
}

pub async fn restore_company(
    key: &String,
    pool: &DbPool,
) -> Result<Company, &'static str> {
    let client = pool.get().await.unwrap();
    let db = client.db(&db_database()).await.unwrap();

    let collection: Collection<ReqwestClient> = db.collection("companies").await.unwrap();
    let data: Company = from_str::<Company>("{\"deleted_at\":null}").unwrap();
    let options: UpdateOptions = UpdateOptions::builder()
        .return_new(true)
        .return_old(true)
        .keep_null(false)
        .build();

    let res: DocumentResponse<Document<Company>> = collection.update_document(key, Document::new(data), options).await.unwrap();
    let record: &Company = res.new_doc().unwrap();
    Ok(record.clone())
}
