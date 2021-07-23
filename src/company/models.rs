use actix_web::{Error, HttpRequest, HttpResponse, Responder};
use anyhow::Result;
use arangors::{AqlQuery, Database};
use futures::future::{ready, Ready};
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use uclient::reqwest::ReqwestClient;

use crate::config::db_database;
use crate::database::{DbConn, DbPool};

#[derive(Debug, Serialize, Deserialize)]
pub struct CompanyResquest {
    pub name: String,
    pub since: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Company {
    pub name: String,
    pub since: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

// implementation of Actix Responder for Company struct so we can return Company from action handler
impl Responder for Company {
    type Error = Error;
    type Future = Ready<Result<HttpResponse, Error>>;

    fn respond_to(self, _req: &HttpRequest) -> Self::Future {
        let body = serde_json::to_string(&self).unwrap();
        // create response and set content type
        ready(
            Ok(
                HttpResponse::Ok()
                    .content_type("application/json")
                    .body(body)
            )
        )
    }
}

// Implementation for Company struct, functions for read/write/update and delete todo from database
impl Company {
    pub fn find_all(pool: &DbPool) -> Result<Vec<Company>, &'static str> {
        let conn: DbConn = pool.get().unwrap();
        let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
        let aql = AqlQuery::builder()
            .query("FOR c IN @@collection LIMIT 10 RETURN c")
            .bind_var("@collection", "companies")
            .build();
        let results: Vec<Company> = db.aql_query(aql).unwrap();
        Ok(results)
    }
}
