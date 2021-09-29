use actix_multipart::{Field, Multipart};
use actix_web::{web, Error, HttpRequest, HttpResponse, Responder};
use arangors::{
    document::{
        options::{InsertOptions, RemoveOptions, UpdateOptions},
        response::DocumentResponse,
    },
    AqlQuery, Collection, Database, Document,
};
use bcrypt::{DEFAULT_COST, hash, verify};
use chrono::prelude::*;
use futures::{StreamExt, TryStreamExt}; // for next or try_next of Multipart
use futures::future::{ready, Ready};
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
pub struct User {
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

impl Clone for User {
    fn clone(&self) -> User {
        User {
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

impl Responder for User {
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

// Implementation for User struct, functions for read/write/update and delete todo from database
impl User {
    pub fn find(params: FindUsersParams, pool: &DbPool) -> Result<Vec<User>, ValidationErrors> {
        let conn: DbConn = pool.get().unwrap();
        let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
        let mut terms = vec!["FOR c IN users"];
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
        let records: Vec<User> = db.aql_query(aql).expect("Query failed");
        Ok(records)
    }

    pub fn show(key: &String, pool: &DbPool) -> Result<User, &'static str> {
        let conn: DbConn = pool.get().unwrap();
        let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
        let collection: Collection<ReqwestClient> = db.collection("users").unwrap();
        let res: Document<User> = collection.document(key.as_ref()).unwrap();
        let record: User = res.document;
        Ok(record)
    }

    pub async fn create(mut payload: Multipart, pool: &DbPool) -> Result<User, Error> {
        let conn: DbConn = pool.get().unwrap();
        let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
        let collection: Collection<ReqwestClient> = db.collection("users").unwrap();
        let now = Utc::now();

        let mut vars: HashMap<String, Value> = HashMap::new();
        while let Ok(Some(mut field)) = payload.try_next().await {
            let content_disposition = field.content_disposition().unwrap();
            println!("content_disposition {}", content_disposition);
            let name = content_disposition.get_name().unwrap();
            println!("name {}", name);
            let content_type = field.content_type();
            println!("content_type {}", content_type);
            match (content_type.type_(), content_type.subtype()) {
                (mime::APPLICATION, mime::OCTET_STREAM) => {
                    let mut body = Vec::with_capacity(512);
                    // field data may be larger than 64KB or it may be on page boundary
                    while let Ok(Some(chunk)) = field.try_next().await {
                        body.extend_from_slice(&chunk);
                    }
                    vars.insert(String::from(name), to_value(str::from_utf8(&body)?)?);
                },
                (mime::IMAGE, _) => {
                    let filename = content_disposition.get_filename().unwrap();
                    println!("filename {}", filename);
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
                    vars.insert(String::from(name), to_value(pathtext)?);
                },
                _ => {}
            }
        }

        let password = vars.get("password").unwrap().to_string();
        let data = User {
            name: Some(vars.get("name").unwrap().to_string()),
            email: Some(vars.get("email").unwrap().to_string()),
            password: Some(hash(password, DEFAULT_COST).unwrap()),
            avatar: Some(vars.get("avatar").unwrap().to_string()),
            created_at: Some(now),
            modified_at: Some(now),
            deleted_at: None,
        };
        // let data = User {
        //     name: Some("qwe".to_string()),
        //     email: Some("fgh@rty.com".to_string()),
        //     password: Some(hash("123456".to_string(), DEFAULT_COST).unwrap()),
        //     avatar: Some("".to_string()),
        //     created_at: Some(now),
        //     modified_at: Some(now),
        //     deleted_at: None,
        // };
        let options: InsertOptions = InsertOptions::builder()
            .return_new(true)
            .build();
        let res: DocumentResponse<Document<User>> = collection.create_document(Document::new(data), options).unwrap();
        let record: &User = res.new_doc().unwrap();
        Ok(record.clone())
    }

    pub fn update(key: &String, payload: &web::Json<User>, pool: &DbPool) -> Result<User, &'static str> {
        let conn: DbConn = pool.get().unwrap();
        let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
        let collection: Collection<ReqwestClient> = db.collection("users").unwrap();
        let obj: Value = json!({
            "modified_at": Utc::now(),
        });
        let text: String = to_string(&obj).unwrap();
        let mut data: User = from_str::<User>(&text).unwrap();
        if payload.name.is_some() {
            data.name = payload.name.clone();
        }
        if payload.email.is_some() {
            data.email = payload.email.clone();
        }
        if payload.password.is_some() {
            data.password = payload.password.clone();
        }
        if payload.avatar.is_some() {
            data.avatar = payload.avatar.clone();
        }
        let options: UpdateOptions = UpdateOptions::builder()
            .return_new(true)
            .return_old(true)
            .build();
        let res: DocumentResponse<Document<User>> = collection.update_document(key, Document::new(data), options).unwrap();
        let record: &User = res.new_doc().unwrap();
        Ok(record.clone())
    }

    pub fn erase(key: &String, pool: &DbPool) -> Result<User, &'static str> {
        let conn: DbConn = pool.get().unwrap();
        let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
        let collection: Collection<ReqwestClient> = db.collection("users").unwrap();
        let options: RemoveOptions = RemoveOptions::builder()
            .return_old(true)
            .build();
        let res: DocumentResponse<Document<User>> = collection.remove_document(key.as_ref(), options, None).unwrap();
        let record: &User = res.old_doc().unwrap();
        Ok(record.clone())
    }

    pub fn trash(key: &String, pool: &DbPool) -> Result<User, &'static str> {
        let conn: DbConn = pool.get().unwrap();
        let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
        let collection: Collection<ReqwestClient> = db.collection("users").unwrap();
        let obj = json!({
            "deleted_at": Utc::now(),
        });
        let text = to_string(&obj).unwrap();
        let data: User = from_str::<User>(&text).unwrap();
        let options: UpdateOptions = UpdateOptions::builder()
            .return_new(true)
            .return_old(true)
            .build();
        let res: DocumentResponse<Document<User>> = collection.update_document(key, Document::new(data), options).unwrap();
        let record: &User = res.new_doc().unwrap();
        Ok(record.clone())
    }

    pub fn restore(key: &String, pool: &DbPool) -> Result<User, &'static str> {
        let conn: DbConn = pool.get().unwrap();
        let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
        let collection: Collection<ReqwestClient> = db.collection("users").unwrap();
        let data: User = from_str::<User>("{\"deleted_at\":null}").unwrap();
        let options: UpdateOptions = UpdateOptions::builder()
            .return_new(true)
            .return_old(true)
            .keep_null(false)
            .build();
        let res: DocumentResponse<Document<User>> = collection.update_document(key, Document::new(data), options).unwrap();
        let record: &User = res.new_doc().unwrap();
        Ok(record.clone())
    }
}
