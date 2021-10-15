use actix_web::{delete, get, post, put, web, Error, HttpResponse};
use validator::Validate;

use crate::company::{
    self,
    Company,
    FindCompaniesParams,
    DeleteCompanyParams,
};
use crate::database::DbPool;

#[get("/companies")]
async fn find(
    payload: web::Query<FindCompaniesParams>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let params: FindCompaniesParams = payload.into_inner();
    match params.validate() {
        Ok(_) => {
            let result = company::find_companies(params, &pool).await.unwrap();
            Ok(HttpResponse::Ok().json(result))
        },
        Err(e) => {
            Ok(HttpResponse::BadRequest().json(e.errors()))
        },
    }
}

#[get("/companies/{key}")]
async fn show(
    key: web::Path<String>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let result = company::show_company(&key, &pool).await.unwrap();
    Ok(HttpResponse::Ok().json(result))
}

#[post("/companies")]
async fn create(
    payload: web::Json<Company>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let result = company::create_company(&payload, &pool).await.unwrap();
    Ok(HttpResponse::Ok().json(result))
}

#[put("/companies/{key}")]
async fn update(
    key: web::Path<String>,
    payload: web::Json<Company>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let result = company::update_company(&key, &payload, &pool).await.unwrap();
    Ok(HttpResponse::Ok().json(result))
}

#[delete("/companies/{key}")]
async fn delete(
    key: web::Path<String>,
    form: web::Form<DeleteCompanyParams>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    match form.mode.as_str() {
        "erase" => {
            let result = company::erase_company(&key, &pool).await.unwrap();
            Ok(HttpResponse::NoContent().json({}))
        },
        "trash" => {
            let result = company::trash_company(&key, &pool).await.unwrap();
            Ok(HttpResponse::Ok().json(result))
        },
        "restore" => {
            let result = company::restore_company(&key, &pool).await.unwrap();
            Ok(HttpResponse::Ok().json(result))
        },
        &_ => {
            Ok(HttpResponse::NoContent().json({}))
        },
    }
}

// function that will be called on new Application to configure routes for this module
pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(find);
    cfg.service(show);
    cfg.service(create);
    cfg.service(update);
    cfg.service(delete);
}
