use actix_multipart::{Multipart};
use actix_web::{web, Error, HttpRequest, HttpResponse, Responder};
use arangors::{
    document::{
        options::{InsertOptions, RemoveOptions, UpdateOptions},
        response::DocumentResponse,
    },
    AqlQuery, Collection, Cursor, Database, Document,
};
use bcrypt::{DEFAULT_COST, hash, verify};
use chrono::prelude::*;
use futures::{
    future::{ready, Ready},
    StreamExt, TryStreamExt, // for next or try_next of Multipart
};
use serde::{Deserialize, Serialize};
use serde_json::{from_str, json, to_string, to_value, Value};
use std::{
    collections::HashMap,
    env,
    fs::File,
    io::Write,
    str,
    vec::Vec,
};
use uclient::reqwest::ReqwestClient;
use validator::{Validate, ValidationError, ValidationErrors};

use crate::config::db_database;
use crate::database::{DbConn, DbPool};

#[derive(Debug, Validate, Deserialize)]
pub struct FindUsersParams {
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

impl Clone for FindUsersParams {
    fn clone(&self) -> FindUsersParams {
        FindUsersParams {
            search: self.search.clone(),
            sort_by: self.sort_by.clone(),
            limit: self.limit.clone(),
        }
    }
}

impl FindUsersParams {
    pub fn check_valid(&self) -> Result<(), ValidationErrors> { // public version of validate
        self.validate()
    }
}

#[derive(Debug, Validate, Deserialize)]
pub struct DeleteUserParams {
    #[validate(custom = "validate_mode")]
    pub mode: String,
}

fn validate_mode(mode: &str) -> Result<(), ValidationError> {
    if mode != "erase" && mode != "trash" && mode != "restore" {
        return Err(ValidationError::new("Wrong mode"));
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct UserRequest {
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub avatar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub created_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub modified_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Clone for UserRequest {
    fn clone(&self) -> UserRequest {
        UserRequest {
            name: self.name.clone(),
            email: self.email.clone(),
            password: self.password.clone(),
            avatar: self.avatar.clone(),
            created_at: self.created_at.clone(),
            modified_at: self.modified_at.clone(),
            deleted_at: self.deleted_at.clone(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserResponse {
    pub _id: String,
    pub _key: String,
    pub _rev: String,
    pub name: String,
    pub email: String,
    pub avatar: String,
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub created_at: Option<DateTime<Utc>>,
    pub modified_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Responder for UserResponse {
    type Error = Error;
    type Future = Ready<Result<HttpResponse, Error>>;

    fn respond_to(self, _req: &HttpRequest) -> Self::Future {
        let body = serde_json::to_string(&self).unwrap();

        // Create response and set content type
        ready(Ok(
            HttpResponse::Ok()
                .content_type("application/json")
                .body(body)
        ))
    }
}

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
                let mut f = web::block(|| File::create(filepath)).await?;
                // field data may be larger than 64KB or it may be on page boundary
                while let Ok(Some(chunk)) = field.try_next().await {
                    f = web::block(move || f.write_all(&chunk).map(|_| f)).await?;
                }
                web::block(move || f.flush()).await?;
                let pathtext = format!("/storage/{}", uniqname);
                vars.insert(String::from(name), pathtext);
            },
            _ => {}
        }
    }

    Ok(vars)
}

// Implementation for read/write/update/delete from database
pub fn find_users(
    params: FindUsersParams,
    pool: &DbPool,
) -> Result<Vec<UserResponse>, ValidationErrors> {
    let conn: DbConn = pool.get().unwrap();
    let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
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
    let records: Vec<UserResponse> = db.aql_query(aql).expect("Query failed");
    Ok(records)
}

pub fn show_user(key: &String, pool: &DbPool) -> Result<UserResponse, &'static str> {
    let conn: DbConn = pool.get().unwrap();
    let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
    let collection: Collection<ReqwestClient> = db.collection("users").unwrap();
    let res: Document<UserResponse> = collection.document(key.as_ref()).unwrap();
    let record: UserResponse = res.document;
    Ok(record)
}

pub async fn create_user(
    payload: Multipart,
    pool: &DbPool,
) -> Result<UserResponse, Error> {
    let conn: DbConn = pool.get().unwrap();
    let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
    let collection: Collection<ReqwestClient> = db.collection("users").unwrap();
    let now = Utc::now();

    let vars: HashMap<String, String> = accept_uploading(payload).await?;

    let password = vars.get("password").unwrap().to_string();
    let data = UserRequest {
        name: Some(vars.get("name").unwrap().to_string()),
        email: Some(vars.get("email").unwrap().to_string()),
        password: Some(hash(password, DEFAULT_COST).unwrap()),
        avatar: Some(vars.get("avatar").unwrap().to_string()),
        created_at: Some(now),
        modified_at: Some(now),
        deleted_at: None,
    };
    let options: InsertOptions = InsertOptions::builder()
        .return_new(true)
        .build();
    let res: DocumentResponse<Document<UserRequest>> = collection.create_document(Document::new(data), options).unwrap();
    let doc: &UserRequest = res.new_doc().unwrap();
    let record: UserRequest = doc.clone();
    let header = res.header().unwrap();
    Ok(UserResponse {
        _id: header._id.clone(),
        _key: header._key.clone(),
        _rev: header._rev.clone(),
        name: record.name.unwrap(),
        email: record.email.unwrap(),
        avatar: record.avatar.unwrap(),
        created_at: record.created_at,
        modified_at: record.modified_at.unwrap(),
        deleted_at: record.deleted_at,
    })
}

pub async fn update_user(
    key: &String,
    payload: Multipart,
    pool: &DbPool,
) -> Result<UserResponse, Error> {
    let conn: DbConn = pool.get().unwrap();
    let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
    let collection: Collection<ReqwestClient> = db.collection("users").unwrap();
    let now = Utc::now();

    let vars: HashMap<String, String> = accept_uploading(payload).await?;

    let password = vars.get("password").unwrap().to_string();
    let data = UserRequest {
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
            Some(hash(password, DEFAULT_COST).unwrap())
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
    let options: UpdateOptions = UpdateOptions::builder()
        .return_new(true)
        .return_old(true)
        .build();
    let res: DocumentResponse<Document<UserRequest>> = collection.update_document(key, Document::new(data), options).unwrap();
    let doc: &UserRequest = res.new_doc().unwrap();
    let record: UserRequest = doc.clone();
    let header = res.header().unwrap();
    Ok(UserResponse {
        _id: header._id.clone(),
        _key: header._key.clone(),
        _rev: header._rev.clone(),
        name: record.name.unwrap(),
        email: record.email.unwrap(),
        avatar: record.avatar.unwrap(),
        created_at: record.created_at,
        modified_at: record.modified_at.unwrap(),
        deleted_at: record.deleted_at,
    })
}

pub fn erase_user(key: &String, pool: &DbPool) -> Result<UserResponse, &'static str> {
    let conn: DbConn = pool.get().unwrap();
    let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
    let collection: Collection<ReqwestClient> = db.collection("users").unwrap();
    let options: RemoveOptions = RemoveOptions::builder()
        .return_old(true)
        .build();
    let res: DocumentResponse<Document<UserRequest>> = collection.remove_document(key.as_ref(), options, None).unwrap();
    let doc: &UserRequest = res.new_doc().unwrap();
    let record: UserRequest = doc.clone();
    let header = res.header().unwrap();
    Ok(UserResponse {
        _id: header._id.clone(),
        _key: header._key.clone(),
        _rev: header._rev.clone(),
        name: record.name.unwrap(),
        email: record.email.unwrap(),
        avatar: record.avatar.unwrap(),
        created_at: record.created_at,
        modified_at: record.modified_at.unwrap(),
        deleted_at: record.deleted_at,
    })
}

pub fn trash_user(key: &String, pool: &DbPool) -> Result<UserResponse, &'static str> {
    let conn: DbConn = pool.get().unwrap();
    let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
    let collection: Collection<ReqwestClient> = db.collection("users").unwrap();
    let obj = json!({
        "deleted_at": Utc::now(),
    });
    let text = to_string(&obj).unwrap();
    let data: UserRequest = from_str::<UserRequest>(&text).unwrap();
    let options: UpdateOptions = UpdateOptions::builder()
        .return_new(true)
        .return_old(true)
        .build();
    let res: DocumentResponse<Document<UserRequest>> = collection.update_document(key, Document::new(data), options).unwrap();
    let doc: &UserRequest = res.new_doc().unwrap();
    let record: UserRequest = doc.clone();
    let header = res.header().unwrap();
    Ok(UserResponse {
        _id: header._id.clone(),
        _key: header._key.clone(),
        _rev: header._rev.clone(),
        name: record.name.unwrap(),
        email: record.email.unwrap(),
        avatar: record.avatar.unwrap(),
        created_at: record.created_at,
        modified_at: record.modified_at.unwrap(),
        deleted_at: record.deleted_at,
    })
}

pub fn restore_user(key: &String, pool: &DbPool) -> Result<UserResponse, &'static str> {
    let conn: DbConn = pool.get().unwrap();
    let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
    let collection: Collection<ReqwestClient> = db.collection("users").unwrap();
    let data: UserRequest = from_str::<UserRequest>("{\"deleted_at\":null}").unwrap();
    let options: UpdateOptions = UpdateOptions::builder()
        .return_new(true)
        .return_old(true)
        .keep_null(false)
        .build();
    let res: DocumentResponse<Document<UserRequest>> = collection.update_document(key, Document::new(data), options).unwrap();
    let doc: &UserRequest = res.new_doc().unwrap();
    let record: UserRequest = doc.clone();
    let header = res.header().unwrap();
    Ok(UserResponse {
        _id: header._id.clone(),
        _key: header._key.clone(),
        _rev: header._rev.clone(),
        name: record.name.unwrap(),
        email: record.email.unwrap(),
        avatar: record.avatar.unwrap(),
        created_at: record.created_at,
        modified_at: record.modified_at.unwrap(),
        deleted_at: record.deleted_at,
    })
}
