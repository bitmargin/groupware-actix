use std::env;

pub fn host() -> String {
  return env::var("HOST").expect("HOST must be set");
}

pub fn port() -> String {
  return env::var("PORT").expect("PORT must be set");
}

pub fn db_host() -> String {
  return env::var("DB_HOST").expect("DB_HOST must be set");
}

pub fn db_port() -> String {
  return env::var("DB_PORT").expect("DB_PORT must be set");
}

pub fn db_username() -> String {
  return env::var("DB_USERNAME").expect("DB_USERNAME must be set");
}

pub fn db_password() -> String {
  return env::var("DB_PASSWORD").expect("DB_PASSWORD must be set");
}

pub fn db_database() -> String {
  return env::var("DB_DATABASE").expect("DB_DATABASE must be set");
}
