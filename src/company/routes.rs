use actix_web::{delete, get, post, put, web, Error, HttpResponse};

use crate::company::{
    Company,
    FindCompaniesParams,
    DeleteCompanyParams,
};
use crate::database::{DbPool};

#[get("/companies")]
async fn find(
    params: web::Query<FindCompaniesParams>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let result = web::block(move || Company::find(&params, &pool)).await?;
    Ok(HttpResponse::Ok().json(result))
}

#[get("/companies/{key}")]
async fn show(
    key: web::Path<String>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let result = web::block(move || Company::show(&key, &pool)).await?;
    Ok(HttpResponse::Ok().json(result))
}

#[post("/companies")]
async fn create(
    params: web::Form<Company>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let result = web::block(move || Company::create(params, &pool)).await?;
    Ok(HttpResponse::Ok().json(result))
}

#[put("/companies/{key}")]
async fn update(
    key: web::Path<String>,
    params: web::Form<Company>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let result = web::block(move || Company::update(&key, params, &pool)).await?;
    Ok(HttpResponse::Ok().json(result))
}

#[delete("/companies/{key}")]
async fn delete(
    key: web::Path<String>,
    params: web::Form<DeleteCompanyParams>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let mode = params.mode.clone();
    let result = web::block(move || {
        if params.mode == "erase" {
            return Company::erase(&key, &pool);
        } else if params.mode == "trash" {
            return Company::trash(&key, &pool);
        } else if params.mode == "restore" {
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
