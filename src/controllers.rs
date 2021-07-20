use actix_web::{get, web, Error, HttpResponse, Responder, Result};
use arangors::{AqlQuery, Database};
use std::vec::Vec;
use uclient::reqwest::ReqwestClient;

use crate::config::{db_database};
use crate::{DbPool, DbConn};
use crate::models::{Company};

#[get("/companies")]
pub async fn get_companies(pool: web::Data<DbPool>) -> Result<HttpResponse, Error> {
    let res = web::block(move || get_companies_pure(pool))
        .await
        .map_err(|e| {
            eprintln!("{}", e);
            HttpResponse::InternalServerError().finish()
        })?;
    Ok(HttpResponse::Ok().json(res))
}

fn get_companies_pure(pool: web::Data<DbPool>) -> Result<Vec<Company>, diesel::result::Error> {
    let conn: DbConn = pool.get().unwrap();
    let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
    let aql = AqlQuery::builder()
        .query("FOR c IN @@collection LIMIT 10 RETURN c")
        .bind_var("@collection", "companies")
        .build();
    let results: Vec<Company> = db.aql_query(aql).unwrap();
    Ok(results)
}

pub async fn get_company_by_id(pool: web::Data<DbPool>) -> impl Responder {
    let conn = pool.get_ref().get().expect("couldn't get db connection from pool");
    format!("hello from get company by id")
}
