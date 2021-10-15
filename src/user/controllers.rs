use actix_multipart::Multipart;
use actix_web::{
    error::ErrorBadRequest,
    web,
    Error,
};
use arangors::{
    connection::ReqwestClient,
    document::{
        options::{InsertOptions, RemoveOptions, UpdateOptions},
        response::DocumentResponse,
    },
    AqlQuery, Collection, Database, Document,
};
use bcrypt::{DEFAULT_COST, hash, verify};
use chrono::prelude::*;
use futures::{StreamExt, TryStreamExt}; // for next or try_next of Multipart
use serde_json::{from_str, json, to_string, to_value, Value};
use std::{
    collections::HashMap,
    env,
    fs::File,
    io::Write,
    str,
    vec::Vec,
};
use validator::{Validate, ValidationErrors};

use crate::config::db_database;
use crate::database::DbPool;
use crate::user::{
    CreateUserRequest,
    FindUsersParams,
    UpdateUserRequest,
    UserResponse,
};

async fn accept_uploading(
    mut payload: Multipart
) -> Result<HashMap<String, String>, Error> {
    let mut vars: HashMap<String, String> = HashMap::new();

    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition().unwrap();
        let name = content_disposition.get_name().unwrap();
        let content_type = field.content_type();

        match (content_type.type_(), content_type.subtype()) {
            (mime::APPLICATION, mime::OCTET_STREAM) => {
                let mut body = Vec::with_capacity(512);
                // field data may be larger than 64KB or it may be on page boundary
                while let Ok(Some(chunk)) = field.try_next().await {
                    body.extend_from_slice(&chunk);
                }
                let val = String::from_utf8(body).unwrap();
                vars.insert(String::from(name), val);
            },
            (mime::IMAGE, _) => {
                let filename = content_disposition.get_filename().unwrap();
                let uniqname = sanitize_filename::sanitize(filename);
                let mut filepath = env::current_dir()?;
                filepath.push("storage");
                filepath.push(&uniqname);
                // field data may be larger than 64KB or it may be on page boundary
                while let Ok(Some(chunk)) = field.try_next().await {
                    tokio::fs::write(&filepath, chunk).await.unwrap();
                }
                let pathtext = format!("/storage/{}", uniqname);
                vars.insert(String::from(name), pathtext);
            },
            _ => {}
        }
    }

    Ok(vars)
}

pub async fn find_users(
    params: FindUsersParams,
    pool: &DbPool,
) -> Result<Vec<UserResponse>, ValidationErrors> {
    let client = pool.get().await.unwrap();
    let db = client.db(&db_database()).await.unwrap();

    let mut terms = vec!["FOR x IN users"];
    let mut vars: HashMap<&str, Value> = HashMap::new();
    if params.search.is_some() {
        let search: String = params.search.unwrap().trim().to_string();
        if !search.is_empty() {
            terms.push("FILTER CONTAINS(x.name, @@search)");
            vars.insert("@search", to_value(search).unwrap());
        }
    }
    if params.sort_by.is_some() {
        let sort_by: String = params.sort_by.unwrap();
        terms.push("SORT x.@@sort_by ASC");
        vars.insert("@sort_by", to_value(sort_by).unwrap());
    }
    if params.limit.is_some() {
        let limit: u32 = params.limit.unwrap();
        terms.push("LIMIT 0, @@limit");
        vars.insert("@limit", to_value(limit).unwrap());
    }

    terms.push("RETURN UNSET(x, 'password')");
    let q = terms.join(" ");

    let aql = AqlQuery::builder()
        .query(&q)
        .bind_vars(vars)
        .build();
    let records: Vec<UserResponse> = db.aql_query(aql).await.unwrap();
    Ok(records)
}

pub async fn show_user(
    key: &String,
    pool: &DbPool,
) -> Result<UserResponse, &'static str> {
    let client = pool.get().await.unwrap();
    let db = client.db(&db_database()).await.unwrap();

    let collection: Collection<ReqwestClient> = db.collection("users").await.unwrap();
    let res: Document<UserResponse> = collection.document(key.as_ref()).await.unwrap();
    let record: UserResponse = res.document;
    Ok(record)
}

pub async fn create_user(
    payload: Multipart,
    pool: &DbPool,
) -> Result<UserResponse, Error> {
    let client = pool.get().await.unwrap();
    let db = client.db(&db_database()).await.unwrap();

    let collection: Collection<ReqwestClient> = db.collection("users").await.unwrap();
    let now = Utc::now();

    let vars: HashMap<String, String> = accept_uploading(payload).await?;
    let mut req = CreateUserRequest {
        name: if vars.contains_key("name") {
            Some(vars.get("name").unwrap().to_string())
        } else {
            None
        },
        email: if vars.contains_key("email") {
            Some(vars.get("email").unwrap().to_string())
        } else {
            None
        },
        password: if vars.contains_key("password") {
            Some(vars.get("password").unwrap().to_string())
        } else {
            None
        },
        password_confirmation: if vars.contains_key("password_confirmation") {
            Some(vars.get("password_confirmation").unwrap().to_string())
        } else {
            None
        },
        avatar: if vars.contains_key("avatar") {
            Some(vars.get("avatar").unwrap().to_string())
        } else {
            None
        },
        created_at: now,
        modified_at: now,
    };

    match req.validate() {
        Ok(_) => {
            req.password = Some(hash(req.password.unwrap(), DEFAULT_COST).unwrap());
            req.password_confirmation = None;

            let options: InsertOptions = InsertOptions::builder()
                .return_new(true)
                .build();
            let res: DocumentResponse<Document<CreateUserRequest>> = collection.create_document(Document::new(req), options).await.unwrap();
            let doc: &CreateUserRequest = res.new_doc().unwrap();
            let record: CreateUserRequest = doc.clone();
            let header = res.header().unwrap();
            Ok(UserResponse {
                _id: header._id.clone(),
                _key: header._key.clone(),
                _rev: header._rev.clone(),
                name: record.name.unwrap(),
                email: record.email.unwrap(),
                avatar: record.avatar.unwrap(),
                created_at: record.created_at,
                modified_at: record.modified_at,
                deleted_at: None,
            })
        },
        Err(e) => {
            let errs = e.field_errors();
            let text = to_string(&errs).unwrap();
            Err(ErrorBadRequest(text))
        }
    }
}

pub async fn update_user(
    key: &String,
    payload: Multipart,
    pool: &DbPool,
) -> Result<UserResponse, Error> {
    let client = pool.get().await.unwrap();
    let db = client.db(&db_database()).await.unwrap();

    let collection: Collection<ReqwestClient> = db.collection("users").await.unwrap();
    let now = Utc::now();

    let vars: HashMap<String, String> = accept_uploading(payload).await?;
    let mut req = UpdateUserRequest {
        name: if vars.contains_key("name") {
            Some(vars.get("name").unwrap().to_string())
        } else {
            None
        },
        email: if vars.contains_key("email") {
            Some(vars.get("email").unwrap().to_string())
        } else {
            None
        },
        password: if vars.contains_key("password") {
            let pswd = vars.get("password").unwrap().to_string();
            Some(hash(pswd, DEFAULT_COST).unwrap())
        } else {
            None
        },
        password_confirmation: if vars.contains_key("password_confirmation") {
            let pswd = vars.get("password_confirmation").unwrap().to_string();
            Some(hash(pswd, DEFAULT_COST).unwrap())
        } else {
            None
        },
        avatar: if vars.contains_key("avatar") {
            Some(vars.get("avatar").unwrap().to_string())
        } else {
            None
        },
        created_at: None,
        modified_at: Some(now),
        deleted_at: None,
    };

    req.validate().map_err(ErrorBadRequest);
    if vars.contains_key("password") {
        req.password = Some(hash(req.password.unwrap(), DEFAULT_COST).unwrap());
        req.password_confirmation = None;
    }

    let options: UpdateOptions = UpdateOptions::builder()
        .return_new(true)
        .return_old(true)
        .build();
    let res: DocumentResponse<Document<UpdateUserRequest>> = collection.update_document(key, Document::new(req), options).await.unwrap();
    let doc: &UpdateUserRequest = res.new_doc().unwrap();
    let record: UpdateUserRequest = doc.clone();
    let header = res.header().unwrap();

    Ok(UserResponse {
        _id: header._id.clone(),
        _key: header._key.clone(),
        _rev: header._rev.clone(),
        name: record.name.unwrap(),
        email: record.email.unwrap(),
        avatar: record.avatar.unwrap(),
        created_at: record.created_at.unwrap(),
        modified_at: record.modified_at.unwrap(),
        deleted_at: record.deleted_at,
    })
}

pub async fn erase_user(
    key: &String,
    pool: &DbPool,
) -> Result<UserResponse, &'static str> {
    let client = pool.get().await.unwrap();
    let db = client.db(&db_database()).await.unwrap();

    let collection: Collection<ReqwestClient> = db.collection("users").await.unwrap();
    let options: RemoveOptions = RemoveOptions::builder()
        .return_old(true)
        .build();

    let res: DocumentResponse<Document<UpdateUserRequest>> = collection.remove_document(key.as_ref(), options, None).await.unwrap();
    let doc: &UpdateUserRequest = res.old_doc().unwrap();
    let record: UpdateUserRequest = doc.clone();
    let header = res.header().unwrap();

    Ok(UserResponse {
        _id: header._id.clone(),
        _key: header._key.clone(),
        _rev: header._rev.clone(),
        name: record.name.unwrap(),
        email: record.email.unwrap(),
        avatar: record.avatar.unwrap(),
        created_at: record.created_at.unwrap(),
        modified_at: record.modified_at.unwrap(),
        deleted_at: record.deleted_at,
    })
}

pub async fn trash_user(
    key: &String,
    pool: &DbPool,
) -> Result<UserResponse, &'static str> {
    let client = pool.get().await.unwrap();
    let db = client.db(&db_database()).await.unwrap();

    let collection: Collection<ReqwestClient> = db.collection("users").await.unwrap();
    let obj = json!({
        "deleted_at": Utc::now(),
    });
    let text = to_string(&obj).unwrap();
    let data: UpdateUserRequest = from_str::<UpdateUserRequest>(&text).unwrap();
    let options: UpdateOptions = UpdateOptions::builder()
        .return_new(true)
        .return_old(true)
        .build();

    let res: DocumentResponse<Document<UpdateUserRequest>> = collection.update_document(key, Document::new(data), options).await.unwrap();
    let doc: &UpdateUserRequest = res.new_doc().unwrap();
    let record: UpdateUserRequest = doc.clone();
    let header = res.header().unwrap();

    Ok(UserResponse {
        _id: header._id.clone(),
        _key: header._key.clone(),
        _rev: header._rev.clone(),
        name: record.name.unwrap(),
        email: record.email.unwrap(),
        avatar: record.avatar.unwrap(),
        created_at: record.created_at.unwrap(),
        modified_at: record.modified_at.unwrap(),
        deleted_at: record.deleted_at,
    })
}

pub async fn restore_user(
    key: &String,
    pool: &DbPool,
) -> Result<UserResponse, &'static str> {
    let client = pool.get().await.unwrap();
    let db = client.db(&db_database()).await.unwrap();

    let collection: Collection<ReqwestClient> = db.collection("users").await.unwrap();
    let data: UpdateUserRequest = from_str::<UpdateUserRequest>("{\"deleted_at\":null}").unwrap();
    let options: UpdateOptions = UpdateOptions::builder()
        .return_new(true)
        .return_old(true)
        .keep_null(false)
        .build();

    let res: DocumentResponse<Document<UpdateUserRequest>> = collection.update_document(key, Document::new(data), options).await.unwrap();
    let doc: &UpdateUserRequest = res.new_doc().unwrap();
    let record: UpdateUserRequest = doc.clone();
    let header = res.header().unwrap();

    Ok(UserResponse {
        _id: header._id.clone(),
        _key: header._key.clone(),
        _rev: header._rev.clone(),
        name: record.name.unwrap(),
        email: record.email.unwrap(),
        avatar: record.avatar.unwrap(),
        created_at: record.created_at.unwrap(),
        modified_at: record.modified_at.unwrap(),
        deleted_at: record.deleted_at,
    })
}
