use actix_web::{error, web, HttpResponse, Responder};
use arangors::{ClientError, Connection, Database};
use r2d2::{ManageConnection, Pool};
use r2d2_arangors::pool::{ArangoDBConnectionManager};
use std::result::Result;

use crate::config::{db_database};

type DbPool = Pool<ArangoDBConnectionManager>;

pub async fn get_companies(conn: web::Data<Connection>) -> impl Responder {
    let db = conn.get_ref().db(&db_database());
    format!("hello from get companies")
}

pub async fn get_company_by_id(pool: web::Data<DbPool>) -> impl Responder {
    let conn = pool.get_ref().get().expect("couldn't get db connection from pool");
    format!("hello from get company by id")
}
