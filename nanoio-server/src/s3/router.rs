use crate::auth::AuthContext;
use crate::domain::StorageError;
use crate::s3::handlers::{bucket, object};
use crate::storage::StorageEngine;
use axum::{
    extract::{Request, State},
    middleware::{from_fn_with_state, Next},
    response::Response,
    routing::{get, put},
    Router,
};
use std::collections::HashMap;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

pub fn create_router<E: StorageEngine>(
    engine: Arc<E>,
    auth: Arc<AuthContext>,
) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .expose_headers(Any);

    let auth_layer = from_fn_with_state(auth.clone(), auth_middleware);

    Router::new()
        // Root: ListBuckets
        .route("/", get(bucket::list_buckets::<E>))
        // Bucket level operations
        .route(
            "/{bucket}",
            put(bucket::create_bucket::<E>)
                .delete(bucket::delete_bucket::<E>)
                .head(bucket::head_bucket::<E>)
                .get(bucket::get_bucket_dispatcher::<E>)
                .post(bucket::post_bucket_dispatcher::<E>),
        )
        // Object level operations (wildcard key support for nested folders)
        .route(
            "/{bucket}/{*key}",
            put(object::put_object::<E>)
                .get(object::get_object::<E>)
                .head(object::head_object::<E>)
                .delete(object::delete_object::<E>)
                .post(object::post_object_dispatcher::<E>),
        )
        .layer(auth_layer)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(engine)
}

async fn auth_middleware(
    State(auth): State<Arc<AuthContext>>,
    req: Request,
    next: Next,
) -> Result<Response, StorageError> {
    let method = req.method().as_str();
    let path = req.uri().path();
    let query_str = req.uri().query().unwrap_or("");
    let query_map: HashMap<String, String> =
        serde_urlencoded::from_str(query_str).unwrap_or_default();

    auth.verify_request(method, path, &query_map, query_str, req.headers())?;

    Ok(next.run(req).await)
}
