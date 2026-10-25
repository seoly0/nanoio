use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use nanoio::{
    auth::{AuthContext, AuthMode},
    create_router,
    storage::fs::FsStorageEngine,
};
use std::sync::Arc;
use tempfile::tempdir;
use tower::ServiceExt;

fn test_app(engine: Arc<FsStorageEngine>, mode: AuthMode) -> axum::Router {
    let auth = Arc::new(AuthContext::new(
        "nanoioadmin".to_string(),
        "nanoioadmin".to_string(),
        mode,
    ));
    create_router(engine, auth)
}

#[tokio::test]
async fn test_full_s3_lifecycle() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let engine = Arc::new(FsStorageEngine::new(tmp.path()).await?);
    let app = test_app(engine, AuthMode::Permissive);

    // 1. Initial ListBuckets -> 빈 목록
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), 10000).await?;
    let body_str = String::from_utf8(body.to_vec())?;
    assert!(body_str.contains("<ListAllMyBucketsResult"));

    // 2. CreateBucket: "test-bucket"
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/test-bucket")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. HeadBucket: 확인
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("HEAD")
                .uri("/test-bucket")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);

    // 4. PutObject: "test-bucket/hello.txt"
    let content = "Hello, Open Object Storage!";
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/test-bucket/hello.txt")
                .header(header::CONTENT_TYPE, "text/plain")
                .header("x-amz-meta-custom-info", "antigravity-rust")
                .body(Body::from(content))?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(resp.headers().contains_key(header::ETAG));

    // 5. HeadObject: "test-bucket/hello.txt"
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("HEAD")
                .uri("/test-bucket/hello.txt")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "text/plain"
    );
    assert_eq!(
        resp.headers().get("x-amz-meta-custom-info").unwrap(),
        "antigravity-rust"
    );
    assert_eq!(
        resp.headers().get(header::CONTENT_LENGTH).unwrap(),
        &content.len().to_string()
    );

    // 6. GetObject: "test-bucket/hello.txt"
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/test-bucket/hello.txt")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), 10000).await?;
    assert_eq!(String::from_utf8(body.to_vec())?, content);

    // 7. GetObject with Range header: "bytes=0-4" ("Hello")
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/test-bucket/hello.txt")
                .header(header::RANGE, "bytes=0-4")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        resp.headers().get(header::CONTENT_RANGE).unwrap(),
        &format!("bytes 0-4/{}", content.len())
    );
    let body = to_bytes(resp.into_body(), 10000).await?;
    assert_eq!(String::from_utf8(body.to_vec())?, "Hello");

    // 8. ListObjectsV2: "test-bucket"
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/test-bucket?list-type=2")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), 10000).await?;
    let body_str = String::from_utf8(body.to_vec())?;
    assert!(body_str.contains("<Key>hello.txt</Key>"));

    // 9. DeleteObject
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/test-bucket/hello.txt")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // 10. DeleteBucket
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/test-bucket")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    Ok(())
}

#[tokio::test]
async fn test_nested_folders_and_prefix_delimiter() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let engine = Arc::new(FsStorageEngine::new(tmp.path()).await?);
    let app = test_app(engine, AuthMode::Permissive);

    // Create bucket
    let _ = app
        .clone()
        .oneshot(Request::builder().method("PUT").uri("/gallery").body(Body::empty())?)
        .await?;

    // Put nested objects
    let files = [
        ("photos/2026/img1.png", "img1-bytes"),
        ("photos/2026/img2.png", "img2-bytes"),
        ("photos/banner.jpg", "banner-bytes"),
        ("readme.md", "docs"),
    ];

    for (k, content) in files {
        let uri = format!("/gallery/{}", k);
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(&uri)
                    .body(Body::from(content))?,
            )
            .await?;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // List with delimiter="/" at root
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/gallery?list-type=2&delimiter=/")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = String::from_utf8(to_bytes(resp.into_body(), 10000).await?.to_vec())?;
    assert!(body.contains("<Prefix>photos/</Prefix>"));
    assert!(body.contains("<Key>readme.md</Key>"));
    assert!(!body.contains("<Key>photos/2026/img1.png</Key>")); // delimiter에 의해 폴더로 접힘

    // List with prefix="photos/" & delimiter="/"
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/gallery?list-type=2&prefix=photos/&delimiter=/")
                .body(Body::empty())?,
        )
        .await?;
    let body = String::from_utf8(to_bytes(resp.into_body(), 10000).await?.to_vec())?;
    assert!(body.contains("<Prefix>photos/2026/</Prefix>"));
    assert!(body.contains("<Key>photos/banner.jpg</Key>"));

    Ok(())
}

#[tokio::test]
async fn test_s3_error_responses() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let engine = Arc::new(FsStorageEngine::new(tmp.path()).await?);
    let app = test_app(engine, AuthMode::Permissive);

    // NoSuchBucket test
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/non-existent/foo.txt")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body = String::from_utf8(to_bytes(resp.into_body(), 10000).await?.to_vec())?;
    assert!(body.contains("<Code>NoSuchBucket</Code>"));

    // Create bucket then NoSuchKey test
    let _ = app
        .clone()
        .oneshot(Request::builder().method("PUT").uri("/valid-bucket").body(Body::empty())?)
        .await?;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/valid-bucket/missing.txt")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body = String::from_utf8(to_bytes(resp.into_body(), 10000).await?.to_vec())?;
    assert!(body.contains("<Code>NoSuchKey</Code>"));

    Ok(())
}

#[tokio::test]
async fn test_multipart_upload_lifecycle() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let engine = Arc::new(FsStorageEngine::new(tmp.path()).await?);
    let app = test_app(engine, AuthMode::Permissive);

    // 1. Create bucket
    let _ = app
        .clone()
        .oneshot(Request::builder().method("PUT").uri("/mp-bucket").body(Body::empty())?)
        .await?;

    // 2. Initiate Multipart Upload
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mp-bucket/large-file.bin?uploads")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);
    let body_str = String::from_utf8(to_bytes(resp.into_body(), 10000).await?.to_vec())?;
    assert!(body_str.contains("<InitiateMultipartUploadResult"));

    // UploadId 추출
    let start_tag = "<UploadId>";
    let end_tag = "</UploadId>";
    let start_idx = body_str.find(start_tag).unwrap() + start_tag.len();
    let end_idx = body_str.find(end_tag).unwrap();
    let upload_id = &body_str[start_idx..end_idx];

    // 3. Upload Part 1
    let part1_content = "Part1Content-";
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/mp-bucket/large-file.bin?partNumber=1&uploadId={}", upload_id))
                .body(Body::from(part1_content))?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);
    let etag1 = resp.headers().get(header::ETAG).unwrap().to_str()?.to_string();

    // 4. Upload Part 2
    let part2_content = "Part2Content!";
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/mp-bucket/large-file.bin?partNumber=2&uploadId={}", upload_id))
                .body(Body::from(part2_content))?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);
    let etag2 = resp.headers().get(header::ETAG).unwrap().to_str()?.to_string();

    // 5. Complete Multipart Upload
    let complete_xml = format!(
        "<CompleteMultipartUpload><Part><PartNumber>1</PartNumber><ETag>{}</ETag></Part><Part><PartNumber>2</PartNumber><ETag>{}</ETag></Part></CompleteMultipartUpload>",
        etag1, etag2
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/mp-bucket/large-file.bin?uploadId={}", upload_id))
                .header(header::CONTENT_TYPE, "application/xml")
                .body(Body::from(complete_xml))?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);
    let body_str = String::from_utf8(to_bytes(resp.into_body(), 10000).await?.to_vec())?;
    assert!(body_str.contains("<CompleteMultipartUploadResult"));

    // 6. GetObject로 병합된 전체 데이터 검증
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/mp-bucket/large-file.bin")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = String::from_utf8(to_bytes(resp.into_body(), 10000).await?.to_vec())?;
    assert_eq!(body, "Part1Content-Part2Content!");

    Ok(())
}

#[tokio::test]
async fn test_object_copy() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let engine = Arc::new(FsStorageEngine::new(tmp.path()).await?);
    let app = test_app(engine, AuthMode::Permissive);

    // Create bucket
    let _ = app
        .clone()
        .oneshot(Request::builder().method("PUT").uri("/source-bucket").body(Body::empty())?)
        .await?;

    // Put original object
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/source-bucket/original.txt")
                .body(Body::from("Original File Content"))?,
        )
        .await?;

    // Copy to new object (same bucket or another bucket)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/source-bucket/copied.txt")
                .header("x-amz-copy-source", "/source-bucket/original.txt")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);
    let body_str = String::from_utf8(to_bytes(resp.into_body(), 10000).await?.to_vec())?;
    assert!(body_str.contains("<CopyObjectResult"));

    // Verify copied object content
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/source-bucket/copied.txt")
                .body(Body::empty())?,
        )
        .await?;
    let body = String::from_utf8(to_bytes(resp.into_body(), 10000).await?.to_vec())?;
    assert_eq!(body, "Original File Content");

    Ok(())
}

#[tokio::test]
async fn test_batch_delete_objects() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let engine = Arc::new(FsStorageEngine::new(tmp.path()).await?);
    let app = test_app(engine, AuthMode::Permissive);

    // Create bucket & 3 objects
    let _ = app
        .clone()
        .oneshot(Request::builder().method("PUT").uri("/batch-del-bucket").body(Body::empty())?)
        .await?;

    for i in 1..=3 {
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/batch-del-bucket/file{}.txt", i))
                    .body(Body::from(format!("content {}", i)))?,
            )
            .await?;
    }

    // Batch delete file1 and file2
    let del_xml = "<Delete><Object><Key>file1.txt</Key></Object><Object><Key>file2.txt</Key></Object></Delete>";
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/batch-del-bucket?delete")
                .header(header::CONTENT_TYPE, "application/xml")
                .body(Body::from(del_xml))?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);
    let body_str = String::from_utf8(to_bytes(resp.into_body(), 10000).await?.to_vec())?;
    assert!(body_str.contains("<Key>file1.txt</Key>"));
    assert!(body_str.contains("<Key>file2.txt</Key>"));

    // file1 should not exist (404)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/batch-del-bucket/file1.txt")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // file3 should still exist (200)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/batch-del-bucket/file3.txt")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);

    Ok(())
}

#[tokio::test]
async fn test_presigned_url_expiry() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let engine = Arc::new(FsStorageEngine::new(tmp.path()).await?);
    let app = test_app(engine, AuthMode::Permissive);

    // Create bucket & object
    let _ = app
        .clone()
        .oneshot(Request::builder().method("PUT").uri("/secure-bucket").body(Body::empty())?)
        .await?;

    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/secure-bucket/secret.txt")
                .body(Body::from("Top Secret"))?,
        )
        .await?;

    // Expired Presigned URL (date 2020-01-01, expires 3600)
    let expired_uri = "/secure-bucket/secret.txt?X-Amz-Date=20200101T000000Z&X-Amz-Expires=3600";
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(expired_uri)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body_str = String::from_utf8(to_bytes(resp.into_body(), 10000).await?.to_vec())?;
    assert!(body_str.contains("<Code>AccessDenied</Code>"));
    assert!(body_str.contains("<Message>Request has expired</Message>"));

    // Valid Presigned URL (date now, expires 3600)
    let now_str = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let valid_uri = format!("/secure-bucket/secret.txt?X-Amz-Date={}&X-Amz-Expires=3600", now_str);
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&valid_uri)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);

    Ok(())
}

#[tokio::test]
async fn test_strict_sigv4_authentication() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let engine = Arc::new(FsStorageEngine::new(tmp.path()).await?);
    let app = test_app(engine, AuthMode::Strict);

    // 1. Strict 모드에서 인증 헤더 없이 요청 -> 403 AccessDenied
    let resp = app
        .clone()
        .oneshot(Request::builder().method("GET").uri("/").body(Body::empty())?)
        .await?;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body = String::from_utf8(to_bytes(resp.into_body(), 10000).await?.to_vec())?;
    assert!(body.contains("<Code>AccessDenied</Code>"));

    // 2. 잘못된 AccessKeyId 제공 -> 403 InvalidAccessKeyId
    let bad_ak_header = "AWS4-HMAC-SHA256 Credential=WRONG_KEY/20261022/us-east-1/s3/aws4_request, SignedHeaders=host, Signature=abcdef";
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/")
                .header(header::AUTHORIZATION, bad_ak_header)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body = String::from_utf8(to_bytes(resp.into_body(), 10000).await?.to_vec())?;
    assert!(body.contains("<Code>InvalidAccessKeyId</Code>"));

    // 3. 올바른 AccessKeyId지만 잘못된 Signature -> 403 SignatureDoesNotMatch
    let bad_sig_header = "AWS4-HMAC-SHA256 Credential=nanoioadmin/20261022/us-east-1/s3/aws4_request, SignedHeaders=host, Signature=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/")
                .header(header::AUTHORIZATION, bad_sig_header)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body = String::from_utf8(to_bytes(resp.into_body(), 10000).await?.to_vec())?;
    assert!(body.contains("<Code>SignatureDoesNotMatch</Code>"));

    // 4. 유효한 SigV4 서명 계산 및 요청 -> 200 OK
    //    Method: GET, Path: /, Query: "", Date: 20261022T000000Z
    //    SignedHeaders: host
    //    Header: host: localhost
    use hex;
    use hmac::{Hmac, Mac};
    use sha2::{Digest, Sha256};
    type HmacSha256 = Hmac<Sha256>;

    let date_stamp = "20261022";
    let amz_date = "20261022T000000Z";
    let region = "us-east-1";
    let service = "s3";

    // Signing Key
    let mut mac = HmacSha256::new_from_slice(b"AWS4nanoioadmin").unwrap();
    mac.update(date_stamp.as_bytes());
    let k_date = mac.finalize().into_bytes();

    let mut mac = HmacSha256::new_from_slice(&k_date).unwrap();
    mac.update(region.as_bytes());
    let k_region = mac.finalize().into_bytes();

    let mut mac = HmacSha256::new_from_slice(&k_region).unwrap();
    mac.update(service.as_bytes());
    let k_service = mac.finalize().into_bytes();

    let mut mac = HmacSha256::new_from_slice(&k_service).unwrap();
    mac.update(b"aws4_request");
    let k_signing = mac.finalize().into_bytes();

    // Canonical Request
    let canonical_req = "GET\n/\n\nhost:localhost\n\nhost\nUNSIGNED-PAYLOAD";
    let mut hasher = Sha256::new();
    hasher.update(canonical_req.as_bytes());
    let hashed_canonical = hex::encode(hasher.finalize());

    // String to Sign
    let scope = format!("{}/{}/{}/aws4_request", date_stamp, region, service);
    let string_to_sign = format!("AWS4-HMAC-SHA256\n{}\n{}\n{}", amz_date, scope, hashed_canonical);

    // Calculate Signature
    let mut mac = HmacSha256::new_from_slice(&k_signing).unwrap();
    mac.update(string_to_sign.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());

    let valid_auth_header = format!(
        "AWS4-HMAC-SHA256 Credential=nanoioadmin/{}, SignedHeaders=host, Signature={}",
        scope, signature
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/")
                .header(header::HOST, "localhost")
                .header("x-amz-date", amz_date)
                .header(header::AUTHORIZATION, valid_auth_header)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = String::from_utf8(to_bytes(resp.into_body(), 10000).await?.to_vec())?;
    assert!(body.contains("<ListAllMyBucketsResult"));

    Ok(())
}

#[tokio::test]
async fn test_iam_multi_user_and_policies() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let engine = Arc::new(FsStorageEngine::new(tmp.path()).await?);

    use nanoio::auth::{IamPolicy, IamUser};
    let mut auth_ctx = AuthContext::new("root".to_string(), "rootsecret".to_string(), AuthMode::Strict);
    auth_ctx.add_user(IamUser {
        access_key_id: "ro_user".to_string(),
        secret_access_key: "ro_secret".to_string(),
        policy: IamPolicy::ReadOnly,
    });
    auth_ctx.add_user(IamUser {
        access_key_id: "rw_user".to_string(),
        secret_access_key: "rw_secret".to_string(),
        policy: IamPolicy::ReadWrite,
    });

    let app = create_router(engine, Arc::new(auth_ctx));

    use hex;
    use hmac::{Hmac, Mac};
    use sha2::{Digest, Sha256};
    type HmacSha256 = Hmac<Sha256>;

    let calc_auth = |method: &str, path: &str, access_key: &str, secret_key: &str| -> (String, String) {
        let date_stamp = "20261022";
        let amz_date = "20261022T000000Z";
        let region = "us-east-1";
        let service = "s3";

        let mut mac = HmacSha256::new_from_slice(format!("AWS4{}", secret_key).as_bytes()).unwrap();
        mac.update(date_stamp.as_bytes());
        let k_date = mac.finalize().into_bytes();

        let mut mac = HmacSha256::new_from_slice(&k_date).unwrap();
        mac.update(region.as_bytes());
        let k_region = mac.finalize().into_bytes();

        let mut mac = HmacSha256::new_from_slice(&k_region).unwrap();
        mac.update(service.as_bytes());
        let k_service = mac.finalize().into_bytes();

        let mut mac = HmacSha256::new_from_slice(&k_service).unwrap();
        mac.update(b"aws4_request");
        let k_signing = mac.finalize().into_bytes();

        let canonical_req = format!("{}\n{}\n\nhost:localhost\n\nhost\nUNSIGNED-PAYLOAD", method, path);
        let mut hasher = Sha256::new();
        hasher.update(canonical_req.as_bytes());
        let hashed_canonical = hex::encode(hasher.finalize());

        let scope = format!("{}/{}/{}/aws4_request", date_stamp, region, service);
        let string_to_sign = format!("AWS4-HMAC-SHA256\n{}\n{}\n{}", amz_date, scope, hashed_canonical);

        let mut mac = HmacSha256::new_from_slice(&k_signing).unwrap();
        mac.update(string_to_sign.as_bytes());
        let sig = hex::encode(mac.finalize().into_bytes());

        let auth_hdr = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders=host, Signature={}",
            access_key, scope, sig
        );
        (amz_date.to_string(), auth_hdr)
    };

    // 1. ro_user: GET / -> 허용 (200 OK)
    let (date, auth) = calc_auth("GET", "/", "ro_user", "ro_secret");
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/")
                .header(header::HOST, "localhost")
                .header("x-amz-date", date)
                .header(header::AUTHORIZATION, auth)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);

    // 2. ro_user: PUT /new-bucket -> 거부 (403 Forbidden)
    let (date, auth) = calc_auth("PUT", "/new-bucket", "ro_user", "ro_secret");
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/new-bucket")
                .header(header::HOST, "localhost")
                .header("x-amz-date", date)
                .header(header::AUTHORIZATION, auth)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body = String::from_utf8(to_bytes(resp.into_body(), 10000).await?.to_vec())?;
    assert!(body.contains("<Code>AccessDenied</Code>"));
    assert!(body.contains("Action 'PUT' is forbidden by IAM policy"));

    // 3. rw_user: PUT /new-bucket -> 허용 (200 OK)
    let (date, auth) = calc_auth("PUT", "/new-bucket", "rw_user", "rw_secret");
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/new-bucket")
                .header(header::HOST, "localhost")
                .header("x-amz-date", date)
                .header(header::AUTHORIZATION, auth)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);

    Ok(())
}

#[tokio::test]
async fn test_security_path_traversal_prevention() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let engine = Arc::new(FsStorageEngine::new(tmp.path()).await?);
    let app = test_app(engine, AuthMode::Permissive);

    // 1. 유효한 버킷 생성
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/safe-bucket")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);

    // 2. 상위 디렉터리 탈출 시도 ("..") -> 400 Bad Request
    let bad_keys = vec![
        "/safe-bucket/../../secret.txt",
        "/safe-bucket/folder/../../../etc/passwd",
        "/safe-bucket/../shadow",
    ];

    for key_uri in bad_keys {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(key_uri)
                    .body(Body::from("malicious content"))?,
            )
            .await?;
        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "Key {} should be rejected with 400 Bad Request",
            key_uri
        );
        let body = String::from_utf8(to_bytes(resp.into_body(), 10000).await?.to_vec())?;
        assert!(body.contains("Key cannot contain '..'"));
    }

    Ok(())
}

#[tokio::test]
async fn test_bucket_naming_rule_edge_cases() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let engine = Arc::new(FsStorageEngine::new(tmp.path()).await?);
    let app = test_app(engine, AuthMode::Permissive);

    let invalid_bucket_names = vec![
        "ab",                   // 2자 이하
        "MyBucketWithCapitals", // 대문자 포함
        "test..bucket",         // 연속된 점
        "test_bucket",          // 언더스코어
        "-leading-hyphen",      // 하이픈으로 시작
        "trailing-hyphen-",     // 하이픈으로 끝남
        "1234567890123456789012345678901234567890123456789012345678901234", // 64자 이상
    ];

    for name in invalid_bucket_names {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/{}", name))
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "Bucket '{}' should be rejected with 400 InvalidBucketName",
            name
        );
        let body = String::from_utf8(to_bytes(resp.into_body(), 10000).await?.to_vec())?;
        assert!(body.contains("<Code>InvalidBucketName</Code>"));
    }

    // 정상 버킷 생성은 성공 (200 OK)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/valid-bucket.123-test")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);

    Ok(())
}

#[tokio::test]
async fn test_http_range_requests_detailed() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let engine = Arc::new(FsStorageEngine::new(tmp.path()).await?);
    let app = test_app(engine, AuthMode::Permissive);

    // 1. 버킷 생성 및 10바이트 파일 업로드: "0123456789"
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/range-bucket")
                .body(Body::empty())?,
        )
        .await?;

    let content = "0123456789";
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/range-bucket/data.txt")
                .body(Body::from(content))?,
        )
        .await?;

    // 2. bytes=0-4 (앞 5바이트 -> "01234")
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/range-bucket/data.txt")
                .header(header::RANGE, "bytes=0-4")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        resp.headers().get(header::CONTENT_RANGE).unwrap(),
        "bytes 0-4/10"
    );
    let body = to_bytes(resp.into_body(), 1000).await?;
    assert_eq!(&body[..], b"01234");

    // 3. bytes=5- (오프셋 5부터 끝까지 -> "56789")
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/range-bucket/data.txt")
                .header(header::RANGE, "bytes=5-")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        resp.headers().get(header::CONTENT_RANGE).unwrap(),
        "bytes 5-9/10"
    );
    let body = to_bytes(resp.into_body(), 1000).await?;
    assert_eq!(&body[..], b"56789");

    // 4. bytes=-3 (마지막 3바이트 suffix -> "789")
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/range-bucket/data.txt")
                .header(header::RANGE, "bytes=-3")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        resp.headers().get(header::CONTENT_RANGE).unwrap(),
        "bytes 7-9/10"
    );
    let body = to_bytes(resp.into_body(), 1000).await?;
    assert_eq!(&body[..], b"789");

    // 5. bytes=20-30 (전체 파일 크기 초과) -> 416 Range Not Satisfiable
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/range-bucket/data.txt")
                .header(header::RANGE, "bytes=20-30")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::RANGE_NOT_SATISFIABLE);

    Ok(())
}

#[tokio::test]
async fn test_multipart_upload_out_of_order_and_abort() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let engine = Arc::new(FsStorageEngine::new(tmp.path()).await?);
    let app = test_app(engine, AuthMode::Permissive);

    // 1. 버킷 생성
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/mp-edge-bucket")
                .body(Body::empty())?,
        )
        .await?;

    // 2. Initiate
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mp-edge-bucket/file.bin?uploads")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);
    let body_str = String::from_utf8(to_bytes(resp.into_body(), 10000).await?.to_vec())?;
    let start_tag = "<UploadId>";
    let end_tag = "</UploadId>";
    let start_idx = body_str.find(start_tag).unwrap() + start_tag.len();
    let end_idx = body_str.find(end_tag).unwrap();
    let upload_id = &body_str[start_idx..end_idx];

    // 3. 순서를 뒤바꿔서 Part 2 먼저 업로드
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/mp-edge-bucket/file.bin?uploadId={}&partNumber=2",
                    upload_id
                ))
                .body(Body::from("WORLD!"))?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);
    let etag_part2 = resp.headers().get(header::ETAG).unwrap().to_str()?.to_string();

    // 4. Part 1 나중에 업로드
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/mp-edge-bucket/file.bin?uploadId={}&partNumber=1",
                    upload_id
                ))
                .body(Body::from("HELLO_"))?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);
    let etag_part1 = resp.headers().get(header::ETAG).unwrap().to_str()?.to_string();

    // 5. Complete with sorted parts (Part 1, Part 2)
    let complete_xml = format!(
        r#"<CompleteMultipartUpload>
            <Part><PartNumber>1</PartNumber><ETag>{}</ETag></Part>
            <Part><PartNumber>2</PartNumber><ETag>{}</ETag></Part>
        </CompleteMultipartUpload>"#,
        etag_part1, etag_part2
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/mp-edge-bucket/file.bin?uploadId={}", upload_id))
                .body(Body::from(complete_xml))?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);

    // 6. GetObject로 순서대로 온전히 병합되었는지 확인 -> "HELLO_WORLD!"
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/mp-edge-bucket/file.bin")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), 1000).await?;
    assert_eq!(&body[..], b"HELLO_WORLD!");

    // 7. AbortMultipartUpload 테스트
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mp-edge-bucket/abort-file.bin?uploads")
                .body(Body::empty())?,
        )
        .await?;
    let body_str = String::from_utf8(to_bytes(resp.into_body(), 10000).await?.to_vec())?;
    let start_idx = body_str.find(start_tag).unwrap() + start_tag.len();
    let end_idx = body_str.find(end_tag).unwrap();
    let abort_upload_id = &body_str[start_idx..end_idx];

    // Abort 실행 -> 204 No Content
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/mp-edge-bucket/abort-file.bin?uploadId={}",
                    abort_upload_id
                ))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Abort 후 다시 해당 uploadId로 파트 업로드 시도 -> 404 NoSuchUpload
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/mp-edge-bucket/abort-file.bin?uploadId={}&partNumber=1",
                    abort_upload_id
                ))
                .body(Body::from("test"))?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    Ok(())
}

#[tokio::test]
async fn test_concurrency_atomic_writes() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let engine = Arc::new(FsStorageEngine::new(tmp.path()).await?);
    let app = test_app(engine, AuthMode::Permissive);

    // 버킷 생성
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/concurrent-bucket")
                .body(Body::empty())?,
        )
        .await?;

    // 10개 태스크가 동시에 같은 키에 서로 다른 문자열을 PUT
    let mut handles = Vec::new();
    for i in 0..10 {
        let app_clone = app.clone();
        let payload = format!("payload_number_{:04}", i);
        handles.push(tokio::spawn(async move {
            let resp = app_clone
                .oneshot(
                    Request::builder()
                        .method("PUT")
                        .uri("/concurrent-bucket/race-key.txt")
                        .body(Body::from(payload.clone()))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
            payload
        }));
    }

    let mut expected_payloads = Vec::new();
    for h in handles {
        expected_payloads.push(h.await?);
    }

    // 최종 GET: 파일이 손상되거나 중간에 잘려 섞이지 않고, 10개 중 온전한 하나의 payload여야 함
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/concurrent-bucket/race-key.txt")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(resp.status(), StatusCode::OK);
    let body_str = String::from_utf8(to_bytes(resp.into_body(), 1000).await?.to_vec())?;
    assert!(
        expected_payloads.contains(&body_str),
        "Resulting body '{}' must be one of the fully written payloads",
        body_str
    );

    Ok(())
}


