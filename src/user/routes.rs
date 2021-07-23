use actix_web::{delete, get, post, put, web, Error, HttpRequest, HttpResponse, Responder};

use crate::company::{Company, CompanyResquest};
use crate::database::{DbConn, DbPool};

#[get("/companies")]
async fn find_all(
    req: HttpRequest,
    pool: web::Data<DbPool>,
) -> impl Responder {
    let result = web::block(move || {
        let conn: DbConn = pool.get().unwrap();
        let db: Database<ReqwestClient> = conn.db(&db_database()).unwrap();
        let aql = AqlQuery::builder()
            .query("FOR c IN @@collection LIMIT 10 RETURN c")
            .bind_var("@collection", "companies")
            .build();
        let results: Vec<Company> = db.aql_query(aql).unwrap();
        Ok::<_, serde_json::error::Error>(serde_json::to_string(&results))
    })
    .await
    .map_err(Error::from)?;

    match result {
        Ok(v) => HttpResponse::Ok().json(v),
        Err(e) => HttpResponse::BadRequest().body("Error trying to read all todos from database"),
    }
}

// function that will be called on new Application to configure routes for this module
pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(find_all);
    // cfg.service(find);
    // cfg.service(create);
    // cfg.service(update);
    // cfg.service(delete);
}
