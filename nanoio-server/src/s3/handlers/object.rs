use crate::auth::validate_presigned_expiry;
use crate::domain::{RangeSpec, StorageError};
use crate::s3::xml::{
    to_s3_xml, XmlCompleteMultipartUploadResult, XmlCopyObjectResult,
    XmlInitiateMultipartUploadResult,
};
use crate::storage::StorageEngine;
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::SecondsFormat;
use futures_util::TryStreamExt;
use http_body_util::BodyStream;
use std::collections::HashMap;
use std::sync::Arc;
use tokio_util::io::{ReaderStream, StreamReader};

pub async fn put_object<E: StorageEngine>(
    State(engine): State<Arc<E>>,
    Path((bucket, key)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    body: Body,
) -> Result<Response, StorageError> {
    // 1. Presigned URL 만료 검증
    validate_presigned_expiry(&params)?;

    // 2. Object Copy (x-amz-copy-source)
    if let Some(src_header) = headers.get("x-amz-copy-source") {
        if let Ok(src_path_str) = src_header.to_str() {
            let clean_src = src_path_str.trim_start_matches('/');
            if let Some((src_b, src_k)) = clean_src.split_once('/') {
                let meta = engine.copy_object(src_b, src_k, &bucket, &key).await?;
                let xml_res = XmlCopyObjectResult::new(meta.last_modified, meta.etag);
                let xml_str = to_s3_xml(&xml_res)?;

                return Ok((
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "application/xml")],
                    xml_str,
                )
                    .into_response());
            }
        }
    }

    // 3. Multipart Upload: UploadPart (partNumber & uploadId)
    if let (Some(part_num_str), Some(upload_id)) = (params.get("partNumber"), params.get("uploadId")) {
        let part_number: u32 = part_num_str
            .parse()
            .map_err(|_| StorageError::BadRequest("Invalid partNumber".to_string()))?;

        let body_stream = BodyStream::new(body)
            .map_ok(|frame| frame.into_data().unwrap_or_default())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));
        let async_reader = StreamReader::new(body_stream);

        let part_info = engine
            .upload_part(&bucket, &key, upload_id, part_number, Box::pin(async_reader))
            .await?;

        return Ok((
            StatusCode::OK,
            [
                (header::ETAG, part_info.etag.as_str()),
                (header::CONTENT_LENGTH, "0"),
            ],
            Body::empty(),
        )
            .into_response());
    }

    // 4. 일반 PutObject
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let mut user_metadata = HashMap::new();
    for (k, v) in &headers {
        let key_str = k.as_str();
        if let Some(meta_key) = key_str.strip_prefix("x-amz-meta-") {
            if let Ok(val_str) = v.to_str() {
                user_metadata.insert(meta_key.to_string(), val_str.to_string());
            }
        }
    }

    let body_stream = BodyStream::new(body)
        .map_ok(|frame| frame.into_data().unwrap_or_default())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));
    let async_reader = StreamReader::new(body_stream);

    let meta = engine
        .put_object(
            &bucket,
            &key,
            Box::pin(async_reader),
            content_type,
            user_metadata,
        )
        .await?;

    Ok((
        StatusCode::OK,
        [
            (header::ETAG, meta.etag.as_str()),
            (header::CONTENT_LENGTH, "0"),
        ],
        Body::empty(),
    )
        .into_response())
}

pub async fn get_object<E: StorageEngine>(
    State(engine): State<Arc<E>>,
    Path((bucket, key)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Result<Response, StorageError> {
    // Presigned URL 만료 검증
    validate_presigned_expiry(&params)?;

    let range = if let Some(r_header) = headers.get(header::RANGE) {
        if let Ok(r_str) = r_header.to_str() {
            Some(RangeSpec::parse(r_str)?)
        } else {
            None
        }
    } else {
        None
    };

    let (meta, stream, content_range) = engine.get_object(&bucket, &key, range).await?;
    let body_stream = ReaderStream::new(stream);
    let body = Body::from_stream(body_stream);

    let mut response_builder = Response::builder()
        .header(header::CONTENT_TYPE, meta.content_type)
        .header(header::ETAG, meta.etag)
        .header(
            header::LAST_MODIFIED,
            meta.last_modified.to_rfc3339_opts(SecondsFormat::Secs, true),
        )
        .header(header::ACCEPT_RANGES, "bytes");

    for (k, v) in meta.user_metadata {
        response_builder = response_builder.header(format!("x-amz-meta-{}", k), v);
    }

    if let Some(cr) = content_range {
        let resp = response_builder
            .status(StatusCode::PARTIAL_CONTENT)
            .header(header::CONTENT_RANGE, cr)
            .body(body)
            .map_err(|e| StorageError::Internal(anyhow::anyhow!(e)))?;
        Ok(resp)
    } else {
        let resp = response_builder
            .status(StatusCode::OK)
            .header(header::CONTENT_LENGTH, meta.size.to_string())
            .body(body)
            .map_err(|e| StorageError::Internal(anyhow::anyhow!(e)))?;
        Ok(resp)
    }
}

pub async fn head_object<E: StorageEngine>(
    State(engine): State<Arc<E>>,
    Path((bucket, key)): Path<(String, String)>,
) -> Result<Response, StorageError> {
    let meta = engine.head_object(&bucket, &key).await?;

    let mut response_builder = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, meta.content_type)
        .header(header::CONTENT_LENGTH, meta.size.to_string())
        .header(header::ETAG, meta.etag)
        .header(
            header::LAST_MODIFIED,
            meta.last_modified.to_rfc3339_opts(SecondsFormat::Secs, true),
        )
        .header(header::ACCEPT_RANGES, "bytes");

    for (k, v) in meta.user_metadata {
        response_builder = response_builder.header(format!("x-amz-meta-{}", k), v);
    }

    let resp = response_builder
        .body(Body::empty())
        .map_err(|e| StorageError::Internal(anyhow::anyhow!(e)))?;
    Ok(resp)
}

pub async fn delete_object<E: StorageEngine>(
    State(engine): State<Arc<E>>,
    Path((bucket, key)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Response, StorageError> {
    // Abort Multipart Upload (?uploadId=...)
    if let Some(upload_id) = params.get("uploadId") {
        engine.abort_multipart_upload(&bucket, &key, upload_id).await?;
        return Ok((StatusCode::NO_CONTENT, Body::empty()).into_response());
    }

    engine.delete_object(&bucket, &key).await?;
    Ok((StatusCode::NO_CONTENT, Body::empty()).into_response())
}

pub async fn post_object_dispatcher<E: StorageEngine>(
    State(engine): State<Arc<E>>,
    Path((bucket, key)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Response, StorageError> {
    // 1. Initiate Multipart Upload (?uploads)
    if params.contains_key("uploads") {
        let content_type = headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let info = engine
            .initiate_multipart_upload(&bucket, &key, content_type)
            .await?;

        let res_xml = XmlInitiateMultipartUploadResult::new(bucket, key, info.upload_id);
        let xml_str = to_s3_xml(&res_xml)?;

        return Ok((
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/xml")],
            xml_str,
        )
            .into_response());
    }

    // 2. Complete Multipart Upload (?uploadId=...)
    if let Some(upload_id) = params.get("uploadId") {
        let complete_req: crate::domain::CompleteMultipartUploadRequest =
            quick_xml::de::from_reader(body.as_ref())
                .map_err(|e| StorageError::BadRequest(format!("Malformed CompleteMultipartUpload XML: {e}")))?;

        let meta = engine
            .complete_multipart_upload(&bucket, &key, upload_id, complete_req.parts)
            .await?;

        let res_xml = XmlCompleteMultipartUploadResult::new(bucket, key, meta.etag);
        let xml_str = to_s3_xml(&res_xml)?;

        return Ok((
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/xml")],
            xml_str,
        )
            .into_response());
    }

    Err(StorageError::BadRequest("Unsupported POST operation on object".to_string()))
}
