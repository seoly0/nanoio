use crate::domain::StorageError;
use crate::s3::xml::{to_s3_xml, ListAllMyBucketsResult, XmlListBucketResult};
use crate::storage::{ListObjectsV2Query, StorageEngine};
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use std::sync::Arc;

pub async fn list_buckets<E: StorageEngine>(
    State(engine): State<Arc<E>>,
) -> Result<Response, StorageError> {
    let buckets = engine.list_buckets().await?;
    let xml_obj = ListAllMyBucketsResult::new(buckets);
    let xml_str = to_s3_xml(&xml_obj)?;

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/xml")],
        xml_str,
    )
        .into_response())
}

pub async fn create_bucket<E: StorageEngine>(
    State(engine): State<Arc<E>>,
    Path(bucket): Path<String>,
) -> Result<Response, StorageError> {
    engine.create_bucket(&bucket).await?;

    Ok((
        StatusCode::OK,
        [
            (header::LOCATION, format!("/{}", bucket).as_str()),
            (header::CONTENT_LENGTH, "0"),
        ],
        Body::empty(),
    )
        .into_response())
}

pub async fn delete_bucket<E: StorageEngine>(
    State(engine): State<Arc<E>>,
    Path(bucket): Path<String>,
) -> Result<Response, StorageError> {
    engine.delete_bucket(&bucket).await?;

    Ok((StatusCode::NO_CONTENT, Body::empty()).into_response())
}

pub async fn head_bucket<E: StorageEngine>(
    State(engine): State<Arc<E>>,
    Path(bucket): Path<String>,
) -> Result<Response, StorageError> {
    let exists = engine.head_bucket(&bucket).await?;
    if exists {
        Ok((StatusCode::OK, Body::empty()).into_response())
    } else {
        Err(StorageError::NoSuchBucket(bucket))
    }
}

pub async fn get_bucket_dispatcher<E: StorageEngine>(
    State(engine): State<Arc<E>>,
    Path(bucket): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Response, StorageError> {
    // S3 ListObjectsV2는 쿼리에 `list-type=2`가 포함되어 있음
    let _list_type = params.get("list-type").map(|s| s.as_str());

    let prefix = params.get("prefix").cloned();
    let delimiter = params.get("delimiter").cloned();
    let max_keys = params.get("max-keys").and_then(|s| s.parse::<usize>().ok());
    let continuation_token = params.get("continuation-token").cloned();
    let start_after = params.get("start-after").cloned();

    let query = ListObjectsV2Query {
        prefix: prefix.clone(),
        delimiter,
        max_keys,
        continuation_token,
        start_after,
    };

    let result = engine.list_objects_v2(&bucket, query).await?;
    let xml_obj = XmlListBucketResult::new(
        bucket,
        prefix.unwrap_or_default(),
        max_keys.unwrap_or(1000),
        result,
    );
    let xml_str = to_s3_xml(&xml_obj)?;

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/xml")],
        xml_str,
    )
        .into_response())
}

pub async fn post_bucket_dispatcher<E: StorageEngine>(
    State(engine): State<Arc<E>>,
    Path(bucket): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
    body: axum::body::Bytes,
) -> Result<Response, StorageError> {
    if params.contains_key("delete") {
        let req_xml: crate::s3::xml::XmlDeleteObjectsRequest = quick_xml::de::from_reader(body.as_ref())
            .map_err(|e| StorageError::BadRequest(format!("Malformed DeleteObjects XML: {e}")))?;

        let keys: Vec<String> = req_xml.objects.into_iter().map(|o| o.key).collect();
        let deleted = engine.delete_objects(&bucket, keys).await?;

        let result_xml = crate::s3::xml::XmlDeleteObjectsResult::new(deleted);
        let xml_str = to_s3_xml(&result_xml)?;

        Ok((
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/xml")],
            xml_str,
        )
            .into_response())
    } else {
        Err(StorageError::BadRequest("Unsupported POST operation on bucket".to_string()))
    }
}
