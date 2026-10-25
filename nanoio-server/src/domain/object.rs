use crate::domain::error::StorageError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectMetadata {
    pub key: String,
    pub size: u64,
    pub etag: String,
    pub content_type: String,
    pub last_modified: DateTime<Utc>,
    pub user_metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectSummary {
    pub key: String,
    pub size: u64,
    pub etag: String,
    pub last_modified: DateTime<Utc>,
    pub storage_class: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RangeSpec {
    /// bytes=start-end (inclusive)
    FromTo(u64, u64),
    /// bytes=start-
    From(u64),
    /// bytes=-suffix_len (마지막 N바이트)
    Suffix(u64),
}

impl RangeSpec {
    /// HTTP Range 헤더 (예: "bytes=0-499") 파싱
    pub fn parse(header_val: &str) -> Result<Self, StorageError> {
        let trimmed = header_val.trim();
        if !trimmed.starts_with("bytes=") {
            return Err(StorageError::InvalidRange);
        }

        let range_part = &trimmed["bytes=".len()..];
        if range_part.is_empty() {
            return Err(StorageError::InvalidRange);
        }

        if let Some(stripped) = range_part.strip_prefix('-') {
            // -suffix_len
            let suffix_len: u64 = stripped.parse().map_err(|_| StorageError::InvalidRange)?;
            return Ok(RangeSpec::Suffix(suffix_len));
        }

        let parts: Vec<&str> = range_part.split('-').collect();
        if parts.len() != 2 {
            return Err(StorageError::InvalidRange);
        }

        let start: u64 = parts[0].parse().map_err(|_| StorageError::InvalidRange)?;
        if parts[1].is_empty() {
            Ok(RangeSpec::From(start))
        } else {
            let end: u64 = parts[1].parse().map_err(|_| StorageError::InvalidRange)?;
            if start > end {
                return Err(StorageError::InvalidRange);
            }
            Ok(RangeSpec::FromTo(start, end))
        }
    }

    /// 파일 전체 크기(file_size)를 바탕으로 실제 [start, length] 및 Content-Range 헤더 계산
    pub fn resolve(&self, file_size: u64) -> Result<(u64, u64, String), StorageError> {
        if file_size == 0 {
            return Err(StorageError::InvalidRange);
        }

        let (start, end) = match self {
            RangeSpec::FromTo(s, e) => {
                let e = (*e).min(file_size - 1);
                if *s > e {
                    return Err(StorageError::InvalidRange);
                }
                (*s, e)
            }
            RangeSpec::From(s) => {
                if *s >= file_size {
                    return Err(StorageError::InvalidRange);
                }
                (*s, file_size - 1)
            }
            RangeSpec::Suffix(len) => {
                let len = (*len).min(file_size);
                (file_size - len, file_size - 1)
            }
        };

        let length = end - start + 1;
        let content_range = format!("bytes {}-{}/{}", start, end, file_size);
        Ok((start, length, content_range))
    }
}

/// S3 키에 대한 안전성 검사 (Path Traversal 방지)
pub fn validate_and_clean_key(key: &str) -> Result<String, StorageError> {
    let clean = key.trim_start_matches('/');
    if clean.is_empty() {
        return Err(StorageError::BadRequest("Key cannot be empty".to_string()));
    }

    // ".." 컴포넌트 차단
    for seg in clean.split('/') {
        if seg == ".." {
            return Err(StorageError::BadRequest("Key cannot contain '..'".to_string()));
        }
    }

    Ok(clean.to_string())
}
