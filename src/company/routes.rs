use actix_web::{delete, get, post, put, web, Error, HttpRequest, HttpResponse};

use crate::company::{Company, CompanyResquest};
use crate::database::{DbPool};

#[get("/companies")]
async fn find_all(
    req: HttpRequest,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let result = web::block(move || Company::find_all(&pool)).await?;
    Ok(HttpResponse::Ok().json(result))
}

// function that will be called on new Application to configure routes for this module
pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(find_all);
    // cfg.service(find);
    // cfg.service(create);
    // cfg.service(update);
    // cfg.service(delete);
}
