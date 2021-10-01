use actix_web::{delete, get, post, put, web, Error, HttpRequest, HttpResponse, Responder};
use actix_multipart::Multipart;

use crate::user::{
    FindUsersParams,
    DeleteUserParams,
    find_users,
    show_user,
    create_user,
    update_user,
    erase_user,
    trash_user,
    restore_user,
};
use crate::database::DbPool;

#[get("/users")]
async fn find(
    payload: web::Query<FindUsersParams>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let params: FindUsersParams = payload.into_inner();
    match params.check_valid() {
        Ok(_) => {
            let result = web::block(move || find_users(params, &pool)).await?;
            Ok(HttpResponse::Ok().json(result))
        },
        Err(e) => {
            Ok(HttpResponse::BadRequest().json(e.errors()))
        },
    }
}

#[get("/users/{key}")]
async fn show(
    web::Path(key): web::Path<String>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let result = web::block(move || show_user(&key, &pool)).await?;
    Ok(HttpResponse::Ok().json(result))
}

#[post("/users")]
async fn create(
    payload: Multipart,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let result = create_user(payload, &pool).await;
    match result {
        Ok(r) => {
            Ok(HttpResponse::Ok().json(r))
        },
        Err(e) => {
            Ok(HttpResponse::BadRequest().json("create user failed"))
        },
    }
}

#[put("/users/{key}")]
async fn update(
    web::Path(key): web::Path<String>,
    payload: Multipart,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let result = update_user(&key, payload, &pool).await;
    match result {
        Ok(r) => {
            Ok(HttpResponse::Ok().json(r))
        },
        Err(e) => {
            Ok(HttpResponse::BadRequest().json("update user failed"))
        },
    }
}

#[delete("/users/{key}")]
async fn delete(
    web::Path(key): web::Path<String>,
    form: web::Form<DeleteUserParams>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let mode = form.mode.clone();
    let result = web::block(move || {
        if form.mode == "erase" {
            return erase_user(&key, &pool);
        } else if form.mode == "trash" {
            return trash_user(&key, &pool);
        } else if form.mode == "restore" {
            return restore_user(&key, &pool);
        }
        return erase_user(&key, &pool);
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
