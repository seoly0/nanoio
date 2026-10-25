use crate::domain::{
    validate_and_clean_key, Bucket, ObjectMetadata, ObjectSummary, RangeSpec, StorageError,
};
use crate::storage::{BoxAsyncRead, ListObjectsV2Query, ListObjectsV2Result, StorageEngine};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use md5::{Digest, Md5};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom};
use tracing::{debug, info};

const META_SUFFIX: &str = ".nanoio_meta";
const BUCKET_META_FILE: &str = ".bucket.json";

#[derive(Clone)]
pub struct FsStorageEngine {
    root_path: Arc<PathBuf>,
}

impl FsStorageEngine {
    pub async fn new<P: AsRef<Path>>(root: P) -> Result<Self, StorageError> {
        let path = root.as_ref().to_path_buf();
        fs::create_dir_all(&path).await?;
        let canonical = fs::canonicalize(&path).await?;
        info!("Initialized FsStorageEngine at {:?}", canonical);
        Ok(Self {
            root_path: Arc::new(canonical),
        })
    }

    fn get_bucket_dir(&self, bucket: &str) -> Result<PathBuf, StorageError> {
        Bucket::validate_name(bucket)?;
        Ok(self.root_path.join(bucket))
    }

    fn get_object_path(&self, bucket_dir: &Path, clean_key: &str) -> PathBuf {
        bucket_dir.join(clean_key)
    }

    fn get_meta_path(&self, object_path: &Path) -> PathBuf {
        let mut p = object_path.as_os_str().to_os_string();
        p.push(META_SUFFIX);
        PathBuf::from(p)
    }

    async fn check_bucket_exists(&self, bucket: &str) -> Result<PathBuf, StorageError> {
        let b_dir = self.get_bucket_dir(bucket)?;
        if !fs::try_exists(&b_dir).await.unwrap_or(false) {
            return Err(StorageError::NoSuchBucket(bucket.to_string()));
        }
        Ok(b_dir)
    }
}

#[async_trait]
impl StorageEngine for FsStorageEngine {
    async fn list_buckets(&self) -> Result<Vec<Bucket>, StorageError> {
        let mut buckets = Vec::new();
        let mut entries = fs::read_dir(&*self.root_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let file_type = entry.file_type().await?;
            if file_type.is_dir() {
                if let Ok(name) = entry.file_name().into_string() {
                    if Bucket::validate_name(&name).is_ok() {
                        let meta_path = entry.path().join(BUCKET_META_FILE);
                        let created_at = if let Ok(content) = fs::read_to_string(&meta_path).await {
                            serde_json::from_str::<DateTime<Utc>>(&content).unwrap_or_else(|_| Utc::now())
                        } else if let Ok(meta) = entry.metadata().await {
                            meta.created()
                                .or_else(|_| meta.modified())
                                .map(|t| DateTime::<Utc>::from(t))
                                .unwrap_or_else(|_| Utc::now())
                        } else {
                            Utc::now()
                        };
                        buckets.push(Bucket::new(name, created_at));
                    }
                }
            }
        }

        buckets.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(buckets)
    }

    async fn create_bucket(&self, name: &str) -> Result<Bucket, StorageError> {
        let b_dir = self.get_bucket_dir(name)?;
        if fs::try_exists(&b_dir).await.unwrap_or(false) {
            return Err(StorageError::BucketAlreadyExists(name.to_string()));
        }

        fs::create_dir_all(&b_dir).await?;
        let now = Utc::now();
        let meta_path = b_dir.join(BUCKET_META_FILE);
        let _ = fs::write(&meta_path, serde_json::to_string(&now).unwrap()).await;

        info!("Created bucket '{}'", name);
        Ok(Bucket::new(name.to_string(), now))
    }

    async fn delete_bucket(&self, name: &str) -> Result<(), StorageError> {
        let b_dir = self.check_bucket_exists(name).await?;

        // 버킷이 비어있는지 확인 (버킷 메타 파일 제외)
        let mut entries = fs::read_dir(&b_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let fname = entry.file_name();
            if fname != BUCKET_META_FILE {
                return Err(StorageError::BucketNotEmpty(name.to_string()));
            }
        }

        fs::remove_dir_all(&b_dir).await?;
        info!("Deleted bucket '{}'", name);
        Ok(())
    }

    async fn head_bucket(&self, name: &str) -> Result<bool, StorageError> {
        let b_dir = self.get_bucket_dir(name)?;
        Ok(fs::try_exists(&b_dir).await.unwrap_or(false))
    }

    async fn put_object(
        &self,
        bucket: &str,
        key: &str,
        mut stream: BoxAsyncRead,
        content_type: Option<String>,
        user_metadata: HashMap<String, String>,
    ) -> Result<ObjectMetadata, StorageError> {
        let b_dir = self.check_bucket_exists(bucket).await?;
        let clean_key = validate_and_clean_key(key)?;
        let obj_path = self.get_object_path(&b_dir, &clean_key);

        if let Some(parent) = obj_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // 원자적 쓰기(Atomic Write)를 위해 임시 파일 사용
        let tmp_filename = format!("{}.tmp.{}", clean_key.replace('/', "_"), uuid_str());
        let tmp_path = b_dir.join(tmp_filename);

        let mut tmp_file = File::create(&tmp_path).await?;
        let mut hasher = Md5::new();
        let mut total_size: u64 = 0;
        let mut buffer = [0u8; 64 * 1024]; // 64KB 청크

        loop {
            let n = stream.read(&mut buffer).await?;
            if n == 0 {
                break;
            }
            tmp_file.write_all(&buffer[..n]).await?;
            hasher.update(&buffer[..n]);
            total_size += n as u64;
        }

        tmp_file.flush().await?;
        drop(tmp_file);

        // 실제 파일 경로로 rename
        fs::rename(&tmp_path, &obj_path).await?;

        let etag = format!("\"{}\"", hex::encode(hasher.finalize()));
        let ctype = content_type
            .or_else(|| {
                mime_guess::from_path(&clean_key)
                    .first_raw()
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "application/octet-stream".to_string());

        let now = Utc::now();
        let meta = ObjectMetadata {
            key: clean_key,
            size: total_size,
            etag: etag.clone(),
            content_type: ctype,
            last_modified: now,
            user_metadata,
        };

        // 메타데이터 파일 저장
        let meta_path = self.get_meta_path(&obj_path);
        let meta_json = serde_json::to_string(&meta).map_err(|e| anyhow::anyhow!(e))?;
        fs::write(&meta_path, meta_json).await?;

        debug!("PutObject bucket='{}', key='{}', size={}", bucket, key, total_size);
        Ok(meta)
    }

    async fn get_object(
        &self,
        bucket: &str,
        key: &str,
        range: Option<RangeSpec>,
    ) -> Result<(ObjectMetadata, BoxAsyncRead, Option<String>), StorageError> {
        let b_dir = self.check_bucket_exists(bucket).await?;
        let clean_key = validate_and_clean_key(key)?;
        let obj_path = self.get_object_path(&b_dir, &clean_key);

        if !fs::try_exists(&obj_path).await.unwrap_or(false) {
            return Err(StorageError::NoSuchKey(clean_key));
        }

        let meta = self.head_object(bucket, &clean_key).await?;
        let mut file = File::open(&obj_path).await?;

        if let Some(spec) = range {
            let (start, length, content_range) = spec.resolve(meta.size)?;
            file.seek(SeekFrom::Start(start)).await?;
            let limited = file.take(length);
            Ok((meta, Box::pin(limited), Some(content_range)))
        } else {
            Ok((meta, Box::pin(file), None))
        }
    }

    async fn head_object(&self, bucket: &str, key: &str) -> Result<ObjectMetadata, StorageError> {
        let b_dir = self.check_bucket_exists(bucket).await?;
        let clean_key = validate_and_clean_key(key)?;
        let obj_path = self.get_object_path(&b_dir, &clean_key);

        if !fs::try_exists(&obj_path).await.unwrap_or(false) {
            return Err(StorageError::NoSuchKey(clean_key));
        }

        let meta_path = self.get_meta_path(&obj_path);
        if let Ok(meta_bytes) = fs::read(&meta_path).await {
            if let Ok(meta) = serde_json::from_slice::<ObjectMetadata>(&meta_bytes) {
                return Ok(meta);
            }
        }

        // 메타데이터 파일이 없거나 파싱 실패 시 실제 파일 속성으로 fallback
        let fs_meta = fs::metadata(&obj_path).await?;
        let size = fs_meta.len();
        let last_modified = fs_meta
            .modified()
            .map(|t| DateTime::<Utc>::from(t))
            .unwrap_or_else(|_| Utc::now());
        let ctype = mime_guess::from_path(&clean_key)
            .first_raw()
            .unwrap_or("application/octet-stream")
            .to_string();

        Ok(ObjectMetadata {
            key: clean_key,
            size,
            etag: format!("\"fallback-{}\"", size),
            content_type: ctype,
            last_modified,
            user_metadata: HashMap::new(),
        })
    }

    async fn delete_object(&self, bucket: &str, key: &str) -> Result<(), StorageError> {
        let b_dir = self.check_bucket_exists(bucket).await?;
        let clean_key = validate_and_clean_key(key)?;
        let obj_path = self.get_object_path(&b_dir, &clean_key);

        if fs::try_exists(&obj_path).await.unwrap_or(false) {
            let _ = fs::remove_file(&obj_path).await;
            let meta_path = self.get_meta_path(&obj_path);
            let _ = fs::remove_file(&meta_path).await;
            debug!("Deleted object bucket='{}', key='{}'", bucket, clean_key);
        }

        Ok(())
    }

    async fn delete_objects(&self, bucket: &str, keys: Vec<String>) -> Result<Vec<String>, StorageError> {
        let mut deleted = Vec::new();
        for key in keys {
            if self.delete_object(bucket, &key).await.is_ok() {
                deleted.push(key);
            }
        }
        Ok(deleted)
    }

    async fn copy_object(
        &self,
        src_bucket: &str,
        src_key: &str,
        dst_bucket: &str,
        dst_key: &str,
    ) -> Result<ObjectMetadata, StorageError> {
        let src_meta = self.head_object(src_bucket, src_key).await?;

        let src_b_dir = self.check_bucket_exists(src_bucket).await?;
        let clean_src_key = validate_and_clean_key(src_key)?;
        let src_path = self.get_object_path(&src_b_dir, &clean_src_key);

        let dst_b_dir = self.check_bucket_exists(dst_bucket).await?;
        let clean_dst_key = validate_and_clean_key(dst_key)?;
        let dst_path = self.get_object_path(&dst_b_dir, &clean_dst_key);

        if let Some(parent) = dst_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::copy(&src_path, &dst_path).await?;

        let now = Utc::now();
        let new_meta = ObjectMetadata {
            key: clean_dst_key,
            size: src_meta.size,
            etag: src_meta.etag,
            content_type: src_meta.content_type,
            last_modified: now,
            user_metadata: src_meta.user_metadata,
        };

        let dst_meta_path = self.get_meta_path(&dst_path);
        let meta_json = serde_json::to_string(&new_meta).map_err(|e| anyhow::anyhow!(e))?;
        fs::write(&dst_meta_path, meta_json).await?;

        info!("Copied object '{}/{}' to '{}/{}'", src_bucket, src_key, dst_bucket, dst_key);
        Ok(new_meta)
    }

    async fn initiate_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
        content_type: Option<String>,
    ) -> Result<crate::domain::MultipartUploadInfo, StorageError> {
        let b_dir = self.check_bucket_exists(bucket).await?;
        let clean_key = validate_and_clean_key(key)?;

        let upload_id = uuid_str();
        let mp_dir = b_dir.join(".multiparts").join(&upload_id);
        fs::create_dir_all(&mp_dir).await?;

        let info = crate::domain::MultipartUploadInfo {
            upload_id: upload_id.clone(),
            bucket: bucket.to_string(),
            key: clean_key,
            initiated: Utc::now(),
            content_type,
        };

        let meta_path = mp_dir.join("info.json");
        fs::write(&meta_path, serde_json::to_string(&info).unwrap()).await?;

        info!("Initiated multipart upload: id='{}', bucket='{}', key='{}'", upload_id, bucket, key);
        Ok(info)
    }

    async fn upload_part(
        &self,
        bucket: &str,
        _key: &str,
        upload_id: &str,
        part_number: u32,
        mut stream: BoxAsyncRead,
    ) -> Result<crate::domain::PartInfo, StorageError> {
        let b_dir = self.check_bucket_exists(bucket).await?;
        let mp_dir = b_dir.join(".multiparts").join(upload_id);
        if !fs::try_exists(&mp_dir).await.unwrap_or(false) {
            return Err(StorageError::NoSuchUpload(upload_id.to_string()));
        }

        let part_path = mp_dir.join(format!("part_{:05}", part_number));
        let mut part_file = File::create(&part_path).await?;
        let mut hasher = Md5::new();
        let mut total_size: u64 = 0;
        let mut buffer = [0u8; 64 * 1024];

        loop {
            let n = stream.read(&mut buffer).await?;
            if n == 0 {
                break;
            }
            part_file.write_all(&buffer[..n]).await?;
            hasher.update(&buffer[..n]);
            total_size += n as u64;
        }

        part_file.flush().await?;
        let etag = format!("\"{}\"", hex::encode(hasher.finalize()));

        debug!("Uploaded part {} for upload_id='{}', size={}", part_number, upload_id, total_size);
        Ok(crate::domain::PartInfo {
            part_number,
            etag,
            size: total_size,
        })
    }

    async fn complete_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
        mut parts: Vec<crate::domain::CompletedPart>,
    ) -> Result<ObjectMetadata, StorageError> {
        let b_dir = self.check_bucket_exists(bucket).await?;
        let clean_key = validate_and_clean_key(key)?;
        let mp_dir = b_dir.join(".multiparts").join(upload_id);

        if !fs::try_exists(&mp_dir).await.unwrap_or(false) {
            return Err(StorageError::NoSuchUpload(upload_id.to_string()));
        }

        let info_path = mp_dir.join("info.json");
        let mp_info: crate::domain::MultipartUploadInfo = if let Ok(data) = fs::read(&info_path).await {
            serde_json::from_slice(&data).map_err(|e| anyhow::anyhow!(e))?
        } else {
            return Err(StorageError::NoSuchUpload(upload_id.to_string()));
        };

        // 파트 번호 오름차순 정렬
        parts.sort_by_key(|p| p.part_number);

        let obj_path = self.get_object_path(&b_dir, &clean_key);
        if let Some(parent) = obj_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let tmp_filename = format!("{}.tmp.{}", clean_key.replace('/', "_"), uuid_str());
        let tmp_path = b_dir.join(tmp_filename);
        let mut merged_file = File::create(&tmp_path).await?;

        let mut overall_hasher = Md5::new();
        let mut part_etags_raw = Vec::new();
        let mut total_size: u64 = 0;
        let mut buffer = [0u8; 64 * 1024];

        for part in &parts {
            let part_path = mp_dir.join(format!("part_{:05}", part.part_number));
            if !fs::try_exists(&part_path).await.unwrap_or(false) {
                let _ = fs::remove_file(&tmp_path).await;
                return Err(StorageError::InvalidPart);
            }

            let mut p_file = File::open(&part_path).await?;
            let mut part_hasher = Md5::new();
            loop {
                let n = p_file.read(&mut buffer).await?;
                if n == 0 {
                    break;
                }
                merged_file.write_all(&buffer[..n]).await?;
                overall_hasher.update(&buffer[..n]);
                part_hasher.update(&buffer[..n]);
                total_size += n as u64;
            }
            part_etags_raw.extend_from_slice(&part_hasher.finalize());
        }

        merged_file.flush().await?;
        drop(merged_file);

        // 원자적 파일 교체
        fs::rename(&tmp_path, &obj_path).await?;

        // S3 표준 멀티파트 ETag 형식: "{md5_of_part_etags}-{parts_count}"
        let mut combined_md5 = Md5::new();
        combined_md5.update(&part_etags_raw);
        let multipart_etag = format!("\"{}-{}\"", hex::encode(combined_md5.finalize()), parts.len());

        let ctype = mp_info.content_type.unwrap_or_else(|| {
            mime_guess::from_path(&clean_key)
                .first_raw()
                .unwrap_or("application/octet-stream")
                .to_string()
        });

        let now = Utc::now();
        let meta = ObjectMetadata {
            key: clean_key,
            size: total_size,
            etag: multipart_etag,
            content_type: ctype,
            last_modified: now,
            user_metadata: HashMap::new(),
        };

        // 메타데이터 파일 저장
        let meta_path = self.get_meta_path(&obj_path);
        let meta_json = serde_json::to_string(&meta).map_err(|e| anyhow::anyhow!(e))?;
        fs::write(&meta_path, meta_json).await?;

        // 멀티파트 임시 디렉터리 정리
        let _ = fs::remove_dir_all(&mp_dir).await;

        info!("Completed multipart upload for '{}/{}', size={}", bucket, key, total_size);
        Ok(meta)
    }

    async fn abort_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
    ) -> Result<(), StorageError> {
        let b_dir = self.check_bucket_exists(bucket).await?;
        let mp_dir = b_dir.join(".multiparts").join(upload_id);
        if fs::try_exists(&mp_dir).await.unwrap_or(false) {
            let _ = fs::remove_dir_all(&mp_dir).await;
            info!("Aborted multipart upload id='{}' for '{}/{}'", upload_id, bucket, key);
        }
        Ok(())
    }

    async fn list_objects_v2(
        &self,
        bucket: &str,
        query: ListObjectsV2Query,
    ) -> Result<ListObjectsV2Result, StorageError> {
        let b_dir = self.check_bucket_exists(bucket).await?;
        let prefix = query.prefix.unwrap_or_default();
        let delimiter = query.delimiter;
        let max_keys = query.max_keys.unwrap_or(1000).min(1000);
        let start_after = query.start_after.or(query.continuation_token);

        let mut all_files = Vec::new();
        collect_all_files(&b_dir, &b_dir, &mut all_files).await?;

        // 정렬
        all_files.sort_by(|a, b| a.cmp(b));

        let mut matched_objects = Vec::new();
        let mut common_prefixes = BTreeSet::new();

        for relative_key in all_files {
            // start_after 필터링
            if let Some(ref start) = start_after {
                if &relative_key <= start {
                    continue;
                }
            }

            // prefix 필터링
            if !relative_key.starts_with(&prefix) {
                continue;
            }

            // delimiter 처리 (디렉터리 가상화)
            if let Some(ref delim) = delimiter {
                let rest = &relative_key[prefix.len()..];
                if let Some(idx) = rest.find(delim) {
                    let common_prefix = format!("{}{}{}", prefix, &rest[..idx], delim);
                    common_prefixes.insert(common_prefix);
                    continue;
                }
            }

            // 개별 오브젝트 메타 수집
            if let Ok(meta) = self.head_object(bucket, &relative_key).await {
                matched_objects.push(ObjectSummary {
                    key: relative_key,
                    size: meta.size,
                    etag: meta.etag,
                    last_modified: meta.last_modified,
                    storage_class: "STANDARD".to_string(),
                });
            }
        }

        let is_truncated = matched_objects.len() > max_keys;
        if is_truncated {
            matched_objects.truncate(max_keys);
        }

        let next_continuation_token = if is_truncated {
            matched_objects.last().map(|o| o.key.clone())
        } else {
            None
        };

        let key_count = matched_objects.len() + common_prefixes.len();

        Ok(ListObjectsV2Result {
            objects: matched_objects,
            common_prefixes: common_prefixes.into_iter().collect(),
            is_truncated,
            next_continuation_token,
            key_count,
        })
    }
}

fn uuid_str() -> String {
    format!("{:x}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0))
}

async fn collect_all_files(
    root: &Path,
    start: &Path,
    results: &mut Vec<String>,
) -> Result<(), StorageError> {
    let mut dir_stack = vec![start.to_path_buf()];

    while let Some(current_dir) = dir_stack.pop() {
        let mut entries = match fs::read_dir(&current_dir).await {
            Ok(e) => e,
            Err(_) => continue,
        };

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let file_type = entry.file_type().await?;

            if file_type.is_dir() {
                if entry.file_name() == ".multiparts" {
                    continue;
                }
                dir_stack.push(path);
            } else if file_type.is_file() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                if file_name == BUCKET_META_FILE
                    || file_name.ends_with(META_SUFFIX)
                    || file_name.contains(".tmp.")
                {
                    continue;
                }

                if let Ok(rel) = path.strip_prefix(root) {
                    let key_str = rel.to_string_lossy().replace('\\', "/");
                    results.push(key_str);
                }
            }
        }
    }

    Ok(())
}
