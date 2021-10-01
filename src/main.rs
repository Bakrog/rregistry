//! # RRegistry
//!
//! `rregistry` is a container registry wrote with rust programming language.
//!
//! **NB! Currently it's a work in progress.**
//!
//! # How to use
//!
//! To use it you need [redis](https://redis.io) (used to search for container
//! manifests) and the following environment variables:
//! - REDIS_CONNECTION_STRING: Connection string to redis, e.g. `redis://localhost:6379`
//! - STORAGE_PATH: Path to store container layers, normally tar or tar.gz files
//!
//! # Roadmap
//! - [x] Add ability to download manifests
//! - [ ] Add ability to download layers
//! - [ ] Add manifest through rest endpoint
//! - [ ] Add layer through rest endpoint
//! - [ ] Add layer redirecting to another service
//! - [ ] Clone manifest from another repository
//! - [ ] Clone layers from another repository
//! - [ ] Implement media type restrictions
//!
//! # Useful links
//! - [Open container distribution specification](https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pull)
//! - [Registry conformance tooling](https://github.com/opencontainers/distribution-spec/tree/main/conformance)

use std::collections::HashMap;
use std::env;

use r2d2::Pool;
use redis::Client;
use rocket::http::Status;
use rocket::serde::{Deserialize, Serialize};
use rocket::{get, launch, routes, Build, Rocket};

static REDIS_CONNECTION_ENV: &str = "REDIS_CONNECTION_STRING";

#[doc(hidden)]
mod blob;
mod manifest;
mod tags;

/// Represents an OCI Content Descriptor
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "rocket::serde", rename_all = "camelCase")]
pub struct Descriptor {
    /// This REQUIRED property contains the media type of the referenced content.
    /// Values MUST comply with [RFC 6838](https://tools.ietf.org/html/rfc6838),
    /// including the naming requirements in its section 4.2.
    pub media_type: String,
    /// This REQUIRED property is the digest of the targeted content, conforming
    /// to the requirements outlined in
    /// [Digests](https://github.com/opencontainers/image-spec/blob/main/descriptor.md#digests).
    /// Retrieved content SHOULD be verified against this digest when consumed
    /// via untrusted sources.
    pub digest: String,
    /// This REQUIRED property specifies the size, in bytes, of the raw content.
    /// This property exists so that a client will have an expected size for the
    /// content before processing. If the length of the retrieved content does not
    /// match the specified length, the content SHOULD NOT be trusted.
    pub size: i64,
    /// This OPTIONAL property specifies a list of URIs from which this object MAY
    /// be downloaded. Each entry MUST conform to [RFC 3986](https://tools.ietf.org/html/rfc3986).
    /// Entries SHOULD use the http and https schemes, as defined in
    /// [RFC 7230](https://tools.ietf.org/html/rfc7230#section-2.7).
    pub urls: Vec<String>,
    /// This OPTIONAL property contains arbitrary metadata for this descriptor.
    /// This OPTIONAL property MUST use the
    /// [annotation rules](https://github.com/opencontainers/image-spec/blob/main/annotations.md#rules).
    pub annotations: HashMap<String, String>,
}

/// To check whether or not the registry implements the OCI Distribution specification
#[get("/")]
async fn v2() -> Status {
    Status::Ok
}

/// Launch website using rocket framework
#[launch]
fn rocket() -> Rocket<Build> {
    rocket::build()
        .mount(
            "/v2",
            routes![
                v2,
                manifest::check_manifest,
                manifest::get_manifest,
                manifest::delete_manifest
            ],
        )
        .manage(create_redis_pool())
}

/// Creates a connection pool to Redis
fn create_redis_pool() -> Pool<Client> {
    let redis_connection_string =
        env::var(REDIS_CONNECTION_ENV).expect("find redis connection string");
    Pool::builder()
        .build(redis::Client::open(redis_connection_string).expect("redis server connection"))
        .expect("redis pool connection")
}

#[doc(hidden)]
#[cfg(test)]
mod test;
