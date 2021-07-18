use chrono::prelude::*;

pub struct Company {
    pub name: String,
    pub since: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

pub struct User {
    pub name: String,
    pub email: String,
    pub password: String,
    pub avatar: String,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}
