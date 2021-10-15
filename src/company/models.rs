use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

#[derive(Clone, Debug, Validate, Deserialize)]
pub struct FindCompaniesParams {
    pub search: Option<String>,
    #[validate(custom = "validate_sort_by")]
    pub sort_by: Option<String>,
    #[validate(range(min = 1, max = 100))]
    pub limit: Option<u32>,
}

fn validate_sort_by(sort_by: &str) -> Result<(), ValidationError> {
    if sort_by != "name" && sort_by != "since" {
        return Err(ValidationError::new("Wrong sort_by"));
    }
    Ok(())
}

#[derive(Debug, Validate, Deserialize)]
pub struct DeleteCompanyParams {
    #[validate(custom = "validate_mode")]
    pub mode: String,
}

fn validate_mode(mode: &str) -> Result<(), ValidationError> {
    if mode != "erase" && mode != "trash" && mode != "restore" {
        return Err(ValidationError::new("Wrong mode"));
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Company {
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub since: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub created_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub modified_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")] // if none, excluded from query
    pub deleted_at: Option<DateTime<Utc>>,
}
