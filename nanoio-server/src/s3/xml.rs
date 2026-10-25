use crate::domain::Bucket;
use crate::storage::ListObjectsV2Result;
use chrono::SecondsFormat;
use serde::{Deserialize, Serialize};

const S3_XMLNS: &str = "http://s3.amazonaws.com/doc/2006-03-01/";

#[derive(Debug, Serialize)]
#[serde(rename = "ListAllMyBucketsResult")]
pub struct ListAllMyBucketsResult {
    #[serde(rename = "@xmlns")]
    pub xmlns: &'static str,
    #[serde(rename = "Owner")]
    pub owner: XmlOwner,
    #[serde(rename = "Buckets")]
    pub buckets: XmlBuckets,
}

#[derive(Debug, Serialize)]
pub struct XmlOwner {
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "DisplayName")]
    pub display_name: String,
}

#[derive(Debug, Serialize)]
pub struct XmlBuckets {
    #[serde(rename = "Bucket")]
    pub bucket: Vec<XmlBucketEntry>,
}

#[derive(Debug, Serialize)]
pub struct XmlBucketEntry {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "CreationDate")]
    pub creation_date: String,
}

impl ListAllMyBucketsResult {
    pub fn new(buckets: Vec<Bucket>) -> Self {
        let entries = buckets
            .into_iter()
            .map(|b| XmlBucketEntry {
                name: b.name,
                creation_date: b.creation_date.to_rfc3339_opts(SecondsFormat::Millis, true),
            })
            .collect();

        Self {
            xmlns: S3_XMLNS,
            owner: XmlOwner {
                id: "nanoio".to_string(),
                display_name: "nanoio".to_string(),
            },
            buckets: XmlBuckets { bucket: entries },
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename = "ListBucketResult")]
pub struct XmlListBucketResult {
    #[serde(rename = "@xmlns")]
    pub xmlns: &'static str,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Prefix")]
    pub prefix: String,
    #[serde(rename = "KeyCount")]
    pub key_count: usize,
    #[serde(rename = "MaxKeys")]
    pub max_keys: usize,
    #[serde(rename = "IsTruncated")]
    pub is_truncated: bool,
    #[serde(rename = "Contents", default, skip_serializing_if = "Vec::is_empty")]
    pub contents: Vec<XmlObjectEntry>,
    #[serde(rename = "CommonPrefixes", default, skip_serializing_if = "Vec::is_empty")]
    pub common_prefixes: Vec<XmlCommonPrefix>,
    #[serde(rename = "NextContinuationToken", skip_serializing_if = "Option::is_none")]
    pub next_continuation_token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct XmlObjectEntry {
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "LastModified")]
    pub last_modified: String,
    #[serde(rename = "ETag")]
    pub etag: String,
    #[serde(rename = "Size")]
    pub size: u64,
    #[serde(rename = "StorageClass")]
    pub storage_class: String,
}

#[derive(Debug, Serialize)]
pub struct XmlCommonPrefix {
    #[serde(rename = "Prefix")]
    pub prefix: String,
}

impl XmlListBucketResult {
    pub fn new(bucket_name: String, prefix: String, max_keys: usize, result: ListObjectsV2Result) -> Self {
        let contents = result
            .objects
            .into_iter()
            .map(|o| XmlObjectEntry {
                key: o.key,
                last_modified: o.last_modified.to_rfc3339_opts(SecondsFormat::Millis, true),
                etag: o.etag,
                size: o.size,
                storage_class: o.storage_class,
            })
            .collect();

        let common_prefixes = result
            .common_prefixes
            .into_iter()
            .map(|p| XmlCommonPrefix { prefix: p })
            .collect();

        Self {
            xmlns: S3_XMLNS,
            name: bucket_name,
            prefix,
            key_count: result.key_count,
            max_keys,
            is_truncated: result.is_truncated,
            contents,
            common_prefixes,
            next_continuation_token: result.next_continuation_token,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename = "InitiateMultipartUploadResult")]
pub struct XmlInitiateMultipartUploadResult {
    #[serde(rename = "@xmlns")]
    pub xmlns: &'static str,
    #[serde(rename = "Bucket")]
    pub bucket: String,
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "UploadId")]
    pub upload_id: String,
}

impl XmlInitiateMultipartUploadResult {
    pub fn new(bucket: String, key: String, upload_id: String) -> Self {
        Self {
            xmlns: S3_XMLNS,
            bucket,
            key,
            upload_id,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename = "CompleteMultipartUploadResult")]
pub struct XmlCompleteMultipartUploadResult {
    #[serde(rename = "@xmlns")]
    pub xmlns: &'static str,
    #[serde(rename = "Location")]
    pub location: String,
    #[serde(rename = "Bucket")]
    pub bucket: String,
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "ETag")]
    pub etag: String,
}

impl XmlCompleteMultipartUploadResult {
    pub fn new(bucket: String, key: String, etag: String) -> Self {
        let location = format!("/{}/{}", bucket, key);
        Self {
            xmlns: S3_XMLNS,
            location,
            bucket,
            key,
            etag,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename = "CopyObjectResult")]
pub struct XmlCopyObjectResult {
    #[serde(rename = "@xmlns")]
    pub xmlns: &'static str,
    #[serde(rename = "LastModified")]
    pub last_modified: String,
    #[serde(rename = "ETag")]
    pub etag: String,
}

impl XmlCopyObjectResult {
    pub fn new(last_modified: chrono::DateTime<chrono::Utc>, etag: String) -> Self {
        Self {
            xmlns: S3_XMLNS,
            last_modified: last_modified.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            etag,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename = "Delete")]
pub struct XmlDeleteObjectsRequest {
    #[serde(rename = "Quiet", default)]
    pub quiet: bool,
    #[serde(rename = "Object", default)]
    pub objects: Vec<XmlDeleteObjectEntry>,
}

#[derive(Debug, Deserialize)]
pub struct XmlDeleteObjectEntry {
    #[serde(rename = "Key")]
    pub key: String,
}

#[derive(Debug, Serialize)]
#[serde(rename = "DeleteResult")]
pub struct XmlDeleteObjectsResult {
    #[serde(rename = "@xmlns")]
    pub xmlns: &'static str,
    #[serde(rename = "Deleted", default, skip_serializing_if = "Vec::is_empty")]
    pub deleted: Vec<XmlDeletedKey>,
}

#[derive(Debug, Serialize)]
pub struct XmlDeletedKey {
    #[serde(rename = "Key")]
    pub key: String,
}

impl XmlDeleteObjectsResult {
    pub fn new(keys: Vec<String>) -> Self {
        let deleted = keys.into_iter().map(|k| XmlDeletedKey { key: k }).collect();
        Self {
            xmlns: S3_XMLNS,
            deleted,
        }
    }
}

pub fn to_s3_xml<T: Serialize>(value: &T) -> Result<String, crate::domain::StorageError> {
    quick_xml::se::to_string(value)
        .map(|xml| format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{xml}"))
        .map_err(|e| crate::domain::StorageError::Internal(anyhow::anyhow!(e)))
}
