use actix_web::{web, Responder};
use r2d2::{Pool};
use r2d2_arangors::pool::{ArangoDBConnectionManager};

type DbPool = Pool<ArangoDBConnectionManager>;

pub async fn get_companies(pool: web::Data<DbPool>) -> impl Responder {
    let conn = pool.get_ref().get().expect("couldn't get db connection from pool");
    format!("hello from get companies")
}

pub async fn get_company_by_id(pool: web::Data<DbPool>) -> impl Responder {
    let conn = pool.get_ref().get().expect("couldn't get db connection from pool");
    format!("hello from get company by id")
}
