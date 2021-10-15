use actix_web::{middleware, web, App, HttpServer};
use dotenv::dotenv;
use std::time::Duration;

mod config;
mod database;
mod company;
mod user;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    println!("Hello, world!");

    let pool = database::init_pool();

    let app = move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(middleware::Logger::default())
            // .wrap(throttle)
            .service(
                web::scope("/api").service(
                    web::scope("/v1")
                        .configure(company::init)
                        .configure(user::init)
                )
            )
    };

    // start http server
    let endpoint = format!("{}:{}", config::host(), config::port());
    HttpServer::new(app)
        .bind(endpoint)?
        .run()
        .await
}
