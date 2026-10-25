use crate::domain::error::StorageError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bucket {
    pub name: String,
    pub creation_date: DateTime<Utc>,
}

impl Bucket {
    pub fn new(name: String, creation_date: DateTime<Utc>) -> Self {
        Self { name, creation_date }
    }

    /// AWS S3 버킷 명명 규칙 검증
    /// - 3 ~ 63자 길이
    /// - 소문자 영문, 숫자, 마침표(.), 하이픈(-)만 허용
    /// - 문자 또는 숫자로 시작하고 끝나야 함
    /// - IP 주소 형식(예: 192.168.5.4) 불가
    pub fn validate_name(name: &str) -> Result<(), StorageError> {
        let len = name.len();
        if !(3..=63).contains(&len) {
            return Err(StorageError::InvalidBucketName(name.to_string()));
        }

        if !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-') {
            return Err(StorageError::InvalidBucketName(name.to_string()));
        }

        let first = name.chars().next().unwrap();
        let last = name.chars().last().unwrap();
        if !(first.is_ascii_alphanumeric() && last.is_ascii_alphanumeric()) {
            return Err(StorageError::InvalidBucketName(name.to_string()));
        }

        if name.contains("..") || name.contains(".-") || name.contains("-.") {
            return Err(StorageError::InvalidBucketName(name.to_string()));
        }

        Ok(())
    }
}
