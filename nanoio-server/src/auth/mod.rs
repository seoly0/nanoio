pub mod sigv4;

pub use sigv4::{AuthContext, AuthMode, IamPolicy, IamUser};

use crate::domain::StorageError;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use std::collections::HashMap;

pub fn validate_presigned_expiry(params: &HashMap<String, String>) -> Result<(), StorageError> {
    let date_str = params
        .get("X-Amz-Date")
        .or_else(|| params.get("x-amz-date"));

    let expires_str = params
        .get("X-Amz-Expires")
        .or_else(|| params.get("x-amz-expires"));

    if let (Some(d_str), Some(e_str)) = (date_str, expires_str) {
        let expires_secs: i64 = e_str
            .parse()
            .map_err(|_| StorageError::BadRequest("Invalid X-Amz-Expires".to_string()))?;

        let parsed_time = NaiveDateTime::parse_from_str(d_str, "%Y%m%dT%H%M%SZ")
            .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
            .or_else(|_| DateTime::parse_from_rfc3339(d_str).map(|dt| dt.with_timezone(&Utc)))
            .map_err(|_| StorageError::BadRequest("Invalid X-Amz-Date format".to_string()))?;

        let expiry_time = parsed_time + Duration::seconds(expires_secs);
        if Utc::now() > expiry_time {
            return Err(StorageError::RequestExpired);
        }
    }

    Ok(())
}
