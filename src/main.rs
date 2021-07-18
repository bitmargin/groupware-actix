use actix_web::{web, App, HttpServer};
use dotenv::dotenv;
use r2d2::{Pool};
use r2d2_arangors::pool::{ArangoDBConnectionManager};
use std::process;
use std::sync::{Arc, Condvar, Mutex};

mod config;
use crate::config::{host, port, db_host, db_port, db_username, db_password};

mod controllers;

type DbPool = Pool<ArangoDBConnectionManager>;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    println!("Hello, world!");

    let pair1 = Arc::new((Mutex::new(false), Condvar::new()));
    let pair2 = Arc::clone(&pair1);
    let pool: DbPool;

    // initialize database connection
    let url = format!("http://{}:{}", db_host(), db_port());
    let manager = ArangoDBConnectionManager::new(&url, &db_username(), &db_password(), false);
    match Pool::builder().max_size(15).build(manager) {
        Ok(conn) => {
            let (lock, cvar) = &*pair2;
            let mut pool_ready = lock.lock().unwrap();
            pool = conn;
            *pool_ready = true;
            cvar.notify_one();
        },
        Err(err) => {
            println!("Error initializing database: {:?}", err);
            process::exit(1);
        },
    }

    // wait for this app to be connected to database
    let (lock, cvar) = &*pair1;
    let mut pool_ready = lock.lock().unwrap();
    while !*pool_ready {
        pool_ready = cvar.wait(pool_ready).unwrap();
    }

    // start http server
    let addr = format!("{}:{}", host(), port());
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/companies", web::get().to(controllers::get_companies))
            .route("/companies/{id}", web::get().to(controllers::get_company_by_id))
    })
    .bind(addr)?
    .run()
    .await
}
