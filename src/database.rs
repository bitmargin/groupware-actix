use mobc::Pool;
use mobc_arangors::ArangoDBConnectionManager;

use crate::config::{db_host, db_port, db_username, db_password};

pub type DbPool = Pool<ArangoDBConnectionManager>;

pub fn init_pool() -> DbPool {
    let url = format!("http://{}:{}", db_host(), db_port());
    let manager = ArangoDBConnectionManager::new(&url, &db_username(), &db_password(), false, false);
    Pool::builder().max_open(15).build(manager)
}
