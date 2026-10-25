use crate::domain::StorageError;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use hex;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthMode {
    Strict,
    Permissive,
}

impl AuthMode {
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "strict" => AuthMode::Strict,
            _ => AuthMode::Permissive,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IamPolicy {
    Admin,
    ReadWrite,
    ReadOnly,
}

impl IamPolicy {
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "readonly" | "read-only" | "ro" => IamPolicy::ReadOnly,
            "readwrite" | "read-write" | "rw" => IamPolicy::ReadWrite,
            _ => IamPolicy::Admin,
        }
    }

    pub fn allows(&self, method: &str) -> bool {
        match self {
            IamPolicy::Admin | IamPolicy::ReadWrite => true,
            IamPolicy::ReadOnly => method.eq_ignore_ascii_case("GET") || method.eq_ignore_ascii_case("HEAD"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IamUser {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub policy: IamPolicy,
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub users: HashMap<String, IamUser>,
    pub mode: AuthMode,
}

impl AuthContext {
    pub fn new(root_access_key_id: String, root_secret_access_key: String, mode: AuthMode) -> Self {
        let mut users = HashMap::new();
        users.insert(
            root_access_key_id.clone(),
            IamUser {
                access_key_id: root_access_key_id,
                secret_access_key: root_secret_access_key,
                policy: IamPolicy::Admin,
            },
        );
        Self { users, mode }
    }

    pub fn add_user(&mut self, user: IamUser) {
        self.users.insert(user.access_key_id.clone(), user);
    }

    pub fn get_user(&self, access_key: &str) -> Option<&IamUser> {
        self.users.get(access_key)
    }

    /// 요청 검증 (Header 방식 또는 Presigned Query 방식)
    pub fn verify_request(
        &self,
        method: &str,
        path: &str,
        query_map: &HashMap<String, String>,
        raw_query_string: &str,
        headers: &axum::http::HeaderMap,
    ) -> Result<(), StorageError> {
        // 1. Presigned Query 방식 확인
        if let Some(cred_param) = query_map.get("X-Amz-Credential").or_else(|| query_map.get("x-amz-credential")) {
            return self.verify_presigned_query(method, path, query_map, raw_query_string, headers, cred_param);
        }

        // 2. Authorization Header 방식 확인
        if let Some(auth_val) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
            if auth_val.starts_with("AWS4-HMAC-SHA256") {
                return self.verify_auth_header(method, path, raw_query_string, headers, auth_val);
            }
        }

        // 3. 인증 정보가 없는 경우:
        //    - Permissive 모드: 통과 (로컬 테스트 및 콘솔 UI)
        //    - Strict 모드: AccessDenied 차단
        if self.mode == AuthMode::Strict {
            return Err(StorageError::AccessDenied("Access Denied: Missing Authorization Header or Presigned Signature".to_string()));
        }

        Ok(())
    }

    fn verify_auth_header(
        &self,
        method: &str,
        path: &str,
        raw_query_string: &str,
        headers: &axum::http::HeaderMap,
        auth_header: &str,
    ) -> Result<(), StorageError> {
        let parsed = parse_auth_header(auth_header)?;

        // IAM Access Key ID 확인
        let user = self.users.get(&parsed.access_key)
            .ok_or_else(|| StorageError::InvalidAccessKeyId(parsed.access_key.clone()))?;

        // IAM Policy 확인 (예: ReadOnly 유저의 쓰기 시도 차단)
        if !user.policy.allows(method) {
            return Err(StorageError::AccessDenied(format!(
                "Action '{}' is forbidden by IAM policy '{:?}'",
                method, user.policy
            )));
        }

        // Request Date
        let req_date = headers
            .get("x-amz-date")
            .or_else(|| headers.get("date"))
            .and_then(|v| v.to_str().ok())
            .unwrap_or(&parsed.date);

        // Signing Key 계산
        let signing_key = derive_signing_key(
            &user.secret_access_key,
            &parsed.date,
            &parsed.region,
            &parsed.service,
        );

        // Canonical Request 생성
        let canonical_req = build_canonical_request(
            method,
            path,
            raw_query_string,
            headers,
            &parsed.signed_headers,
            headers.get("x-amz-content-sha256").and_then(|v| v.to_str().ok()).unwrap_or("UNSIGNED-PAYLOAD"),
        );

        let hashed_canonical = sha256_hex(canonical_req.as_bytes());
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{}",
            req_date, parsed.scope, hashed_canonical
        );

        let expected_signature = hmac_sha256_hex(&signing_key, string_to_sign.as_bytes());

        if expected_signature.eq_ignore_ascii_case(&parsed.signature) {
            Ok(())
        } else {
            Err(StorageError::SignatureDoesNotMatch)
        }
    }

    fn verify_presigned_query(
        &self,
        method: &str,
        path: &str,
        query_map: &HashMap<String, String>,
        raw_query_string: &str,
        headers: &axum::http::HeaderMap,
        credential: &str,
    ) -> Result<(), StorageError> {
        let parts: Vec<&str> = credential.split('/').collect();
        if parts.len() < 5 {
            return Err(StorageError::BadRequest("Malformed X-Amz-Credential".to_string()));
        }

        let access_key = parts[0];
        let date = parts[1];
        let region = parts[2];
        let service = parts[3];

        let user = self.users.get(access_key)
            .ok_or_else(|| StorageError::InvalidAccessKeyId(access_key.to_string()))?;

        // IAM Policy 확인
        if !user.policy.allows(method) {
            return Err(StorageError::AccessDenied(format!(
                "Action '{}' is forbidden by IAM policy '{:?}'",
                method, user.policy
            )));
        }

        // 만료 시간(TTL) 검증
        let date_str = query_map.get("X-Amz-Date").or_else(|| query_map.get("x-amz-date"))
            .ok_or_else(|| StorageError::BadRequest("Missing X-Amz-Date".to_string()))?;
        let expires_str = query_map.get("X-Amz-Expires").or_else(|| query_map.get("x-amz-expires"))
            .ok_or_else(|| StorageError::BadRequest("Missing X-Amz-Expires".to_string()))?;

        let expires_secs: i64 = expires_str.parse().map_err(|_| StorageError::BadRequest("Invalid X-Amz-Expires".to_string()))?;
        let parsed_time = NaiveDateTime::parse_from_str(date_str, "%Y%m%dT%H%M%SZ")
            .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
            .or_else(|_| DateTime::parse_from_rfc3339(date_str).map(|dt| dt.with_timezone(&Utc)))
            .map_err(|_| StorageError::BadRequest("Invalid X-Amz-Date format".to_string()))?;

        if Utc::now() > parsed_time + Duration::seconds(expires_secs) {
            return Err(StorageError::RequestExpired);
        }

        let client_signature = query_map.get("X-Amz-Signature").or_else(|| query_map.get("x-amz-signature"))
            .ok_or_else(|| StorageError::BadRequest("Missing X-Amz-Signature".to_string()))?;

        let signed_headers_str = query_map.get("X-Amz-SignedHeaders").or_else(|| query_map.get("x-amz-signed-headers"))
            .map(|s| s.as_str())
            .unwrap_or("host");

        let signed_headers: Vec<&str> = signed_headers_str.split(';').collect();

        let signing_key = derive_signing_key(&user.secret_access_key, date, region, service);

        // Presigned Query 문자열에서 X-Amz-Signature 제외한 canonical query string 생성
        let canonical_query = build_canonical_query_for_presigned(raw_query_string);

        let canonical_req = format!(
            "{}\n{}\n{}\n{}\n{}\nUNSIGNED-PAYLOAD",
            method,
            path,
            canonical_query,
            build_canonical_headers(headers, &signed_headers),
            signed_headers_str
        );

        let hashed_canonical = sha256_hex(canonical_req.as_bytes());
        let scope = format!("{}/{}/{}/aws4_request", date, region, service);
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{}",
            date_str, scope, hashed_canonical
        );

        let expected_signature = hmac_sha256_hex(&signing_key, string_to_sign.as_bytes());

        if expected_signature.eq_ignore_ascii_case(client_signature) {
            Ok(())
        } else {
            Err(StorageError::SignatureDoesNotMatch)
        }
    }
}

struct ParsedAuthHeader<'a> {
    access_key: String,
    date: String,
    region: String,
    service: String,
    scope: String,
    signed_headers: Vec<&'a str>,
    signature: String,
}

fn parse_auth_header<'a>(header: &'a str) -> Result<ParsedAuthHeader<'a>, StorageError> {
    let parts: Vec<&str> = header.split(',').map(|s| s.trim()).collect();
    let mut credential = "";
    let mut signed_headers = "";
    let mut signature = "";

    for part in parts {
        if part.starts_with("AWS4-HMAC-SHA256 Credential=") {
            credential = &part["AWS4-HMAC-SHA256 Credential=".len()..];
        } else if let Some(stripped) = part.strip_prefix("Credential=") {
            credential = stripped;
        } else if let Some(stripped) = part.strip_prefix("SignedHeaders=") {
            signed_headers = stripped;
        } else if let Some(stripped) = part.strip_prefix("Signature=") {
            signature = stripped;
        }
    }

    if credential.is_empty() || signed_headers.is_empty() || signature.is_empty() {
        return Err(StorageError::BadRequest("Malformed Authorization header".to_string()));
    }

    let cred_parts: Vec<&str> = credential.split('/').collect();
    if cred_parts.len() < 5 {
        return Err(StorageError::BadRequest("Malformed Credential in Authorization header".to_string()));
    }

    let access_key = cred_parts[0].to_string();
    let date = cred_parts[1].to_string();
    let region = cred_parts[2].to_string();
    let service = cred_parts[3].to_string();
    let scope = cred_parts[1..].join("/");
    let signed_headers_vec: Vec<&'a str> = signed_headers.split(';').collect();

    Ok(ParsedAuthHeader {
        access_key,
        date,
        region,
        service,
        scope,
        signed_headers: signed_headers_vec,
        signature: signature.to_string(),
    })
}

fn derive_signing_key(secret: &str, date: &str, region: &str, service: &str) -> Vec<u8> {
    let k_secret = format!("AWS4{}", secret);
    let k_date = hmac_sha256(k_secret.as_bytes(), date.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    hmac_sha256(&k_service, b"aws4_request")
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn hmac_sha256_hex(key: &[u8], data: &[u8]) -> String {
    hex::encode(hmac_sha256(key, data))
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

fn build_canonical_headers(headers: &axum::http::HeaderMap, signed_headers: &[&str]) -> String {
    let mut list = Vec::new();
    for name in signed_headers {
        let lower = name.to_lowercase();
        if let Some(val) = headers.get(&lower).and_then(|v| v.to_str().ok()) {
            list.push(format!("{}:{}\n", lower, val.trim()));
        }
    }
    list.join("")
}

fn build_canonical_request(
    method: &str,
    path: &str,
    raw_query: &str,
    headers: &axum::http::HeaderMap,
    signed_headers: &[&str],
    payload_hash: &str,
) -> String {
    let canonical_headers = build_canonical_headers(headers, signed_headers);
    let signed_headers_str = signed_headers.join(";");

    format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        method, path, raw_query, canonical_headers, signed_headers_str, payload_hash
    )
}

fn build_canonical_query_for_presigned(raw_query: &str) -> String {
    let mut params: Vec<(&str, &str)> = raw_query
        .split('&')
        .filter_map(|pair| {
            if pair.is_empty() {
                return None;
            }
            let mut it = pair.splitn(2, '=');
            let k = it.next()?;
            let v = it.next().unwrap_or("");
            if k.eq_ignore_ascii_case("X-Amz-Signature") {
                None
            } else {
                Some((k, v))
            }
        })
        .collect();

    params.sort_by(|a, b| a.0.cmp(b.0));
    params
        .into_iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&")
}
