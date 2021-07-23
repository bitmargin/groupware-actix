use actix_web::{Result};
use r2d2::{Pool, PooledConnection, Error};
use r2d2_arangors::pool::{ArangoDBConnectionManager};

use crate::config::{db_host, db_port, db_username, db_password};

pub type DbPool = Pool<ArangoDBConnectionManager>;
pub type DbConn = PooledConnection<ArangoDBConnectionManager>;

pub fn init_pool() -> Result<DbPool, Error> {
    let url = format!("http://{}:{}", db_host(), db_port());
    let manager = ArangoDBConnectionManager::new(&url, &db_username(), &db_password(), false);
    Pool::builder().max_size(15).build(manager)
}
