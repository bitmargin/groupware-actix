use actix_ratelimit::{MemoryStore, MemoryStoreActor, RateLimiter};
use actix_web::{middleware, web, App, HttpServer};
use dotenv::dotenv;
use std::time::Duration;

mod config;
mod database;
mod company;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    println!("Hello, world!");

    let pool = database::init_pool().expect("Failed to create pool");

    let app = move || {
        // initialize store of rate limit
        let store = MemoryStore::new();

        // Register the middleware
        // which allows for a maximum of
        // 100 requests per minute per client
        // based on IP address
        let throttle = RateLimiter::new(
            MemoryStoreActor::from(store.clone()).start()
        )
        .with_interval(Duration::from_secs(60))
        .with_max_requests(100);
        // .with_identifier(|req| {
        //     let key = req.headers().get("x-api-key").unwrap();
        //     let key = key.to_str().unwrap();
        //     Ok(key.to_string())
        // });

        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(middleware::Logger::default())
            .wrap(throttle)
            .service(
                web::scope("/api").service(
                    web::scope("/v1")
                        .configure(company::init)
                )
            )
    };

    // start http server
    let addr = format!("{}:{}", config::host(), config::port());
    HttpServer::new(app)
        .bind(addr)?
        .run()
        .await
}
