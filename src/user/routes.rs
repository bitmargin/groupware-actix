use actix_web::{delete, get, post, put, web, Error, HttpRequest, HttpResponse, Responder};
use actix_multipart::Multipart;

use crate::user::{
    User,
    FindUsersParams,
    DeleteUserParams,
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
            let result = web::block(move || User::find(params, &pool)).await?;
            Ok(HttpResponse::Ok().json(result))
        },
        Err(e) => {
            Ok(HttpResponse::BadRequest().json(e.errors()))
        },
    }
}

#[post("/users")]
async fn create(
    payload: Multipart,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let result = User::create(payload, &pool).await;
    match result {
        Ok(r) => {
            Ok(HttpResponse::Ok().json(r))
        },
        Err(e) => {
            Ok(HttpResponse::BadRequest().json("create user failed"))
        },
    }
}

// function that will be called on new Application to configure routes for this module
pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(find);
    // cfg.service(show);
    cfg.service(create);
    // cfg.service(update);
    // cfg.service(delete);
}
