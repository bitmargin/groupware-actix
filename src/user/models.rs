use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::str;
use validator::{Validate, ValidationError, ValidationErrors};

#[derive(Clone, Debug, Validate, Deserialize)]
pub struct FindUsersParams {
    pub search: Option<String>,
    #[validate(custom = "validate_sort_by")]
    pub sort_by: Option<String>,
    #[validate(range(min = 1, max = 100))]
    pub limit: Option<u32>,
}

fn validate_sort_by(sort_by: &str) -> Result<(), ValidationError> {
    match sort_by {
        "name" | "since" => Ok(()),
        _ => Err(ValidationError::new("Wrong sort_by")),
    }
}

#[derive(Debug, Validate, Deserialize)]
pub struct DeleteUserParams {
    #[validate(custom = "validate_mode")]
    pub mode: String,
}

fn validate_mode(mode: &str) -> Result<(), ValidationError> {
    match mode {
        "erase" | "trash" | "restore" => Ok(()),
        _ => Err(ValidationError::new("Wrong mode")),
    }
}

#[derive(Clone, Debug, Validate, Serialize, Deserialize)]
pub struct CreateUserRequest {
    #[validate(required)]
    pub name: Option<String>,
    #[validate(required, email)]
    pub email: Option<String>,
    #[validate(required, length(min = 6))]
    pub password: Option<String>,
    #[validate(required, must_match = "password")]
    pub password_confirmation: Option<String>,
    #[validate(required)]
    pub avatar: Option<String>,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Validate, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub name: Option<String>,
    #[validate(email)]
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub email: Option<String>,
    #[validate(length(min = 6))]
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub password: Option<String>,
    #[validate(must_match = "password")]
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub password_confirmation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub avatar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub created_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub modified_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserResponse {
    pub _id: String,
    pub _key: String,
    pub _rev: String,
    pub name: String,
    pub email: String,
    pub avatar: String,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub deleted_at: Option<DateTime<Utc>>,
}
