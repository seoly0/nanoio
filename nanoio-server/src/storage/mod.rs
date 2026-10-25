pub mod fs;

use crate::domain::{Bucket, ObjectMetadata, ObjectSummary, RangeSpec, StorageError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;
use tokio::io::AsyncRead;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ListObjectsV2Query {
    pub prefix: Option<String>,
    pub delimiter: Option<String>,
    pub max_keys: Option<usize>,
    pub continuation_token: Option<String>,
    pub start_after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListObjectsV2Result {
    pub objects: Vec<ObjectSummary>,
    pub common_prefixes: Vec<String>,
    pub is_truncated: bool,
    pub next_continuation_token: Option<String>,
    pub key_count: usize,
}

pub type BoxAsyncRead = Pin<Box<dyn AsyncRead + Send>>;

#[async_trait]
pub trait StorageEngine: Send + Sync + 'static {
    async fn list_buckets(&self) -> Result<Vec<Bucket>, StorageError>;

    async fn create_bucket(&self, name: &str) -> Result<Bucket, StorageError>;

    async fn delete_bucket(&self, name: &str) -> Result<(), StorageError>;

    async fn head_bucket(&self, name: &str) -> Result<bool, StorageError>;

    async fn put_object(
        &self,
        bucket: &str,
        key: &str,
        stream: BoxAsyncRead,
        content_type: Option<String>,
        user_metadata: HashMap<String, String>,
    ) -> Result<ObjectMetadata, StorageError>;

    async fn get_object(
        &self,
        bucket: &str,
        key: &str,
        range: Option<RangeSpec>,
    ) -> Result<(ObjectMetadata, BoxAsyncRead, Option<String>), StorageError>;

    async fn head_object(&self, bucket: &str, key: &str) -> Result<ObjectMetadata, StorageError>;

    async fn delete_object(&self, bucket: &str, key: &str) -> Result<(), StorageError>;

    async fn delete_objects(&self, bucket: &str, keys: Vec<String>) -> Result<Vec<String>, StorageError>;

    async fn copy_object(
        &self,
        src_bucket: &str,
        src_key: &str,
        dst_bucket: &str,
        dst_key: &str,
    ) -> Result<ObjectMetadata, StorageError>;

    async fn initiate_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
        content_type: Option<String>,
    ) -> Result<crate::domain::MultipartUploadInfo, StorageError>;

    async fn upload_part(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
        part_number: u32,
        stream: BoxAsyncRead,
    ) -> Result<crate::domain::PartInfo, StorageError>;

    async fn complete_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
        parts: Vec<crate::domain::CompletedPart>,
    ) -> Result<ObjectMetadata, StorageError>;

    async fn abort_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
    ) -> Result<(), StorageError>;

    async fn list_objects_v2(
        &self,
        bucket: &str,
        query: ListObjectsV2Query,
    ) -> Result<ListObjectsV2Result, StorageError>;
}
