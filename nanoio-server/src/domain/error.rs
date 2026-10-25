use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use quick_xml::se::to_string;
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("The specified bucket does not exist.")]
    NoSuchBucket(String),

    #[error("The specified key does not exist.")]
    NoSuchKey(String),

    #[error("The requested bucket name is not available.")]
    BucketAlreadyExists(String),

    #[error("The bucket you tried to delete is not empty.")]
    BucketNotEmpty(String),

    #[error("Invalid bucket name.")]
    InvalidBucketName(String),

    #[error("The requested range is not satisfiable.")]
    InvalidRange,

    #[error("Request has expired.")]
    RequestExpired,

    #[error("The specified multipart upload does not exist.")]
    NoSuchUpload(String),

    #[error("One or more of the specified parts could not be found or is invalid.")]
    InvalidPart,

    #[error("The AWS Access Key Id you provided does not exist in our records.")]
    InvalidAccessKeyId(String),

    #[error("The request signature we calculated does not match the signature you provided.")]
    SignatureDoesNotMatch,

    #[error("Access Denied: {0}")]
    AccessDenied(String),

    #[error("Invalid request: {0}")]
    BadRequest(String),

    #[error("Internal storage error: {0}")]
    Internal(#[from] anyhow::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Serialize)]
#[serde(rename = "Error")]
pub struct S3XmlError {
    #[serde(rename = "Code")]
    pub code: String,
    #[serde(rename = "Message")]
    pub message: String,
    #[serde(rename = "Resource", skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
    #[serde(rename = "RequestId")]
    pub request_id: String,
}

impl StorageError {
    pub fn to_s3_error(&self, resource: Option<String>) -> (StatusCode, S3XmlError) {
        let request_id = uuid_or_timestamp();
        match self {
            StorageError::NoSuchBucket(name) => (
                StatusCode::NOT_FOUND,
                S3XmlError {
                    code: "NoSuchBucket".to_string(),
                    message: format!("The specified bucket '{name}' does not exist."),
                    resource: resource.or_else(|| Some(name.clone())),
                    request_id,
                },
            ),
            StorageError::NoSuchKey(key) => (
                StatusCode::NOT_FOUND,
                S3XmlError {
                    code: "NoSuchKey".to_string(),
                    message: format!("The specified key '{key}' does not exist."),
                    resource: resource.or_else(|| Some(key.clone())),
                    request_id,
                },
            ),
            StorageError::BucketAlreadyExists(name) => (
                StatusCode::CONFLICT,
                S3XmlError {
                    code: "BucketAlreadyExists".to_string(),
                    message: format!("The requested bucket name '{name}' is not available."),
                    resource: resource.or_else(|| Some(name.clone())),
                    request_id,
                },
            ),
            StorageError::BucketNotEmpty(name) => (
                StatusCode::CONFLICT,
                S3XmlError {
                    code: "BucketNotEmpty".to_string(),
                    message: "The bucket you tried to delete is not empty.".to_string(),
                    resource: resource.or_else(|| Some(name.clone())),
                    request_id,
                },
            ),
            StorageError::InvalidBucketName(name) => (
                StatusCode::BAD_REQUEST,
                S3XmlError {
                    code: "InvalidBucketName".to_string(),
                    message: format!("The specified bucket name '{name}' is invalid."),
                    resource: resource.or_else(|| Some(name.clone())),
                    request_id,
                },
            ),
            StorageError::InvalidRange => (
                StatusCode::RANGE_NOT_SATISFIABLE,
                S3XmlError {
                    code: "InvalidRange".to_string(),
                    message: "The requested range is not satisfiable".to_string(),
                    resource,
                    request_id,
                },
            ),
            StorageError::RequestExpired => (
                StatusCode::FORBIDDEN,
                S3XmlError {
                    code: "AccessDenied".to_string(),
                    message: "Request has expired".to_string(),
                    resource,
                    request_id,
                },
            ),
            StorageError::NoSuchUpload(upload_id) => (
                StatusCode::NOT_FOUND,
                S3XmlError {
                    code: "NoSuchUpload".to_string(),
                    message: format!("The specified multipart upload with ID '{upload_id}' does not exist."),
                    resource,
                    request_id,
                },
            ),
            StorageError::InvalidPart => (
                StatusCode::BAD_REQUEST,
                S3XmlError {
                    code: "InvalidPart".to_string(),
                    message: "One or more of the specified parts could not be found or is invalid.".to_string(),
                    resource,
                    request_id,
                },
            ),
            StorageError::InvalidAccessKeyId(key_id) => (
                StatusCode::FORBIDDEN,
                S3XmlError {
                    code: "InvalidAccessKeyId".to_string(),
                    message: "The AWS Access Key Id you provided does not exist in our records.".to_string(),
                    resource: resource.or_else(|| Some(key_id.clone())),
                    request_id,
                },
            ),
            StorageError::SignatureDoesNotMatch => (
                StatusCode::FORBIDDEN,
                S3XmlError {
                    code: "SignatureDoesNotMatch".to_string(),
                    message: "The request signature we calculated does not match the signature you provided.".to_string(),
                    resource,
                    request_id,
                },
            ),
            StorageError::AccessDenied(msg) => (
                StatusCode::FORBIDDEN,
                S3XmlError {
                    code: "AccessDenied".to_string(),
                    message: msg.clone(),
                    resource,
                    request_id,
                },
            ),
            StorageError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                S3XmlError {
                    code: "InvalidArgument".to_string(),
                    message: msg.clone(),
                    resource,
                    request_id,
                },
            ),
            StorageError::Internal(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                S3XmlError {
                    code: "InternalError".to_string(),
                    message: format!("We encountered an internal error: {err}"),
                    resource,
                    request_id,
                },
            ),
            StorageError::Io(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                S3XmlError {
                    code: "InternalError".to_string(),
                    message: format!("Underlying storage I/O error: {err}"),
                    resource,
                    request_id,
                },
            ),
        }
    }
}

fn uuid_or_timestamp() -> String {
    format!("{:x}", chrono::Utc::now().timestamp_micros())
}

impl IntoResponse for StorageError {
    fn into_response(self) -> Response {
        let (status, err_xml) = self.to_s3_error(None);
        let xml_body = match to_string(&err_xml) {
            Ok(xml) => format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{xml}"),
            Err(_) => {
                r#"<?xml version="1.0" encoding="UTF-8"?><Error><Code>InternalError</Code><Message>Failed to serialize XML</Message></Error>"#.to_string()
            }
        };

        (
            status,
            [
                (axum::http::header::CONTENT_TYPE, "application/xml"),
                (axum::http::header::HeaderName::from_static("x-amz-request-id"), err_xml.request_id.as_str()),
            ],
            xml_body,
        )
            .into_response()
    }
}
