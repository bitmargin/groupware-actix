use actix_web::{delete, get, post, put, web, Error, HttpResponse};

use crate::company::{
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
    match params.check_valid() {
        Ok(_) => {
            let result = web::block(move || Company::find(params, &pool)).await?;
            Ok(HttpResponse::Ok().json(result))
        },
        Err(e) => {
            Ok(HttpResponse::BadRequest().json(e.errors()))
        },
    }
}

#[get("/companies/{key}")]
async fn show(
    web::Path(key): web::Path<String>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let result = web::block(move || Company::show(&key, &pool)).await?;
    Ok(HttpResponse::Ok().json(result))
}

#[post("/companies")]
async fn create(
    payload: web::Json<Company>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let result = web::block(move || Company::create(&payload, &pool)).await?;
    Ok(HttpResponse::Ok().json(result))
}

#[put("/companies/{key}")]
async fn update(
    web::Path(key): web::Path<String>,
    payload: web::Json<Company>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let result = web::block(move || Company::update(&key, &payload, &pool)).await?;
    Ok(HttpResponse::Ok().json(result))
}

#[delete("/companies/{key}")]
async fn delete(
    web::Path(key): web::Path<String>,
    form: web::Form<DeleteCompanyParams>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let mode = form.mode.clone();
    let result = web::block(move || {
        if form.mode == "erase" {
            return Company::erase(&key, &pool);
        } else if form.mode == "trash" {
            return Company::trash(&key, &pool);
        } else if form.mode == "restore" {
            return Company::restore(&key, &pool);
        }
        return Company::erase(&key, &pool);
    }).await?;
    if mode == "erase" {
        return Ok(HttpResponse::NoContent().json({}));
    } else {
        return Ok(HttpResponse::Ok().json(result));
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
