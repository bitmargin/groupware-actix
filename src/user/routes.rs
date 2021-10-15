use actix_web::{delete, get, post, put, web, Error, HttpRequest, HttpResponse, Responder};
use actix_multipart::Multipart;
use serde_json::{from_str, json, Value};
use validator::Validate;

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
    match params.validate() {
        Ok(_) => {
            let result = find_users(params, &pool).await.unwrap();
            Ok(HttpResponse::Ok().json(result))
        },
        Err(e) => {
            Ok(HttpResponse::BadRequest().json(e.errors()))
        },
    }
}

#[get("/users/{key}")]
async fn show(
    key: web::Path<String>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let result = show_user(&key, &pool).await.unwrap();
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
            let reason: Value = from_str(e.to_string().as_str()).unwrap();
            let obj = json!({
                "success": false,
                "messages": reason,
            });
            Ok(HttpResponse::BadRequest().json(obj))
        },
    }
}

#[put("/users/{key}")]
async fn update(
    key: web::Path<String>,
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
    key: web::Path<String>,
    form: web::Form<DeleteUserParams>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    match form.mode.as_str() {
        "erase" => {
            let result = erase_user(&key, &pool).await.unwrap();
            Ok(HttpResponse::NoContent().json({}))
        },
        "trash" => {
            let result = trash_user(&key, &pool).await.unwrap();
            Ok(HttpResponse::Ok().json(result))
        },
        "restore" => {
            let result = restore_user(&key, &pool).await.unwrap();
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
