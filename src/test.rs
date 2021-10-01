use super::{rocket, Descriptor, REDIS_CONNECTION_ENV};
// use super::blob::Blob;
use super::manifest::Manifest;

use std::env;

use rocket::http::Status;
use rocket::local::asynchronous::Client;
use rocket::serde::json::serde_json;

use redis::{Client as redis_client, Commands};

use testcontainers::clients::Cli;
use testcontainers::images::redis::Redis as RedisImage;
use testcontainers::{clients, core::RunArgs, images::redis as redis_image, Container, Docker};
// use tempfile::tempdir;
// use std::path::Path;

const REDIS_PORT: u16 = 6379;
const DEFAULT_DIGEST: &str = "sha256:default_digest";

#[tokio::test]
async fn implements_oci_v2() {
    let docker_client = docker_client();
    let redis = run_redis(&docker_client).await;
    let host_redis_port = get_host_port(&redis).unwrap();
    let _connection_string = set_redis_connection_environment_variable(host_redis_port);
    let client = Client::tracked(rocket())
        .await
        .expect("valid rocket instance");
    let response = client.get("/v2/").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
}

#[tokio::test]
async fn manifest_doesnt_exist() {
    let docker_client = docker_client();
    let redis = run_redis(&docker_client).await;
    let host_redis_port = get_host_port(&redis).unwrap();
    let _connection_string = set_redis_connection_environment_variable(host_redis_port);
    let client = Client::tracked(rocket())
        .await
        .expect("valid rocket instance");
    let response = client
        .head("/v2/test/manifests/dont_exist")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::NotFound);
}

#[tokio::test]
async fn manifest_does_exist() {
    let docker_client = docker_client();
    let redis = run_redis(&docker_client).await;
    let host_redis_port = get_host_port(&redis).unwrap();
    let connection_string = set_redis_connection_environment_variable(host_redis_port);
    let manifest_name = "test";
    let manifest_reference = "exists";
    let manifest = generate_manifest_body(DEFAULT_DIGEST);
    add_manifest(
        manifest_name,
        manifest_reference,
        &manifest,
        connection_string,
    );
    let client = Client::tracked(rocket())
        .await
        .expect("valid rocket instance");
    let uri = format!("/v2/{}/manifests/{}", manifest_name, manifest_reference);
    let response = client.head(uri).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
}

#[tokio::test]
async fn manifest_does_exist_by_digest() {
    let docker_client = docker_client();
    let redis = run_redis(&docker_client).await;
    let host_redis_port = get_host_port(&redis).unwrap();
    let connection_string = set_redis_connection_environment_variable(host_redis_port);
    let manifest_name = "test";
    let manifest_reference = "exists";
    let manifest = generate_manifest_body(DEFAULT_DIGEST);
    add_manifest(
        manifest_name,
        manifest_reference,
        &manifest,
        connection_string,
    );
    let client = Client::tracked(rocket())
        .await
        .expect("valid rocket instance");
    let uri = format!("/v2/{}/manifests/{}", manifest_name, DEFAULT_DIGEST);
    let response = client.head(uri).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
}

#[tokio::test]
async fn manifest_can_be_downloaded() {
    let docker_client = docker_client();
    let redis = run_redis(&docker_client).await;
    let host_redis_port = get_host_port(&redis).unwrap();
    let connection_string = set_redis_connection_environment_variable(host_redis_port);
    let manifest_name = "test";
    let manifest_reference = "exists";
    let manifest = generate_manifest_body(DEFAULT_DIGEST);
    add_manifest(
        manifest_name,
        manifest_reference,
        &manifest,
        connection_string,
    );
    let client = Client::tracked(rocket())
        .await
        .expect("valid rocket instance");
    let uri = format!("/v2/{}/manifests/{}", manifest_name, manifest_reference);
    let response = client.get(uri).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(
        response.into_string().await.unwrap(),
        serde_json::to_string(&manifest).unwrap()
    );
}

#[tokio::test]
async fn manifest_can_be_downloaded_by_digest() {
    let docker_client = docker_client();
    let redis = run_redis(&docker_client).await;
    let host_redis_port = get_host_port(&redis).unwrap();
    let connection_string = set_redis_connection_environment_variable(host_redis_port);
    let manifest_name = "test";
    let manifest_reference = "exists";
    let manifest = generate_manifest_body(DEFAULT_DIGEST);
    add_manifest(
        manifest_name,
        manifest_reference,
        &manifest,
        connection_string,
    );
    let client = Client::tracked(rocket())
        .await
        .expect("valid rocket instance");
    let uri = format!("/v2/{}/manifests/{}", manifest_name, DEFAULT_DIGEST);
    let response = client.get(uri).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(
        response.into_string().await.unwrap(),
        serde_json::to_string(&manifest).unwrap()
    );
}

#[tokio::test]
async fn manifest_that_doesnt_exists_cant_be_deleted_by_tag() {
    let docker_client = docker_client();
    let redis = run_redis(&docker_client).await;
    let host_redis_port = get_host_port(&redis).unwrap();
    let _connection_string = set_redis_connection_environment_variable(host_redis_port);
    let client = Client::tracked(rocket())
        .await
        .expect("valid rocket instance");
    let response = client
        .delete("/v2/test/manifests/dont_exist")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::NotFound);
}

#[tokio::test]
async fn manifest_that_doesnt_exists_cant_be_deleted_by_digest() {
    let docker_client = docker_client();
    let redis = run_redis(&docker_client).await;
    let host_redis_port = get_host_port(&redis).unwrap();
    let _connection_string = set_redis_connection_environment_variable(host_redis_port);
    let client = Client::tracked(rocket())
        .await
        .expect("valid rocket instance");
    let response = client
        .delete("/v2/test/manifests/sha256:encoded_sha")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::NotFound);
}

#[tokio::test]
async fn manifest_can_be_deleted_by_tag() {
    let docker_client = docker_client();
    let redis = run_redis(&docker_client).await;
    let host_redis_port = get_host_port(&redis).unwrap();
    let connection_string = set_redis_connection_environment_variable(host_redis_port);
    let manifest_name = "test";
    let manifest_reference = "exists";
    let manifest = generate_manifest_body(DEFAULT_DIGEST);
    add_manifest(
        manifest_name,
        manifest_reference,
        &manifest,
        connection_string,
    );
    let client = Client::tracked(rocket())
        .await
        .expect("valid rocket instance");
    let uri = format!("/v2/{}/manifests/{}", manifest_name, manifest_reference);
    let response = client.delete(uri).dispatch().await;
    assert_eq!(response.status(), Status::Accepted);
}

#[tokio::test]
async fn manifest_can_be_deleted_by_digest() {
    let docker_client = docker_client();
    let redis = run_redis(&docker_client).await;
    let host_redis_port = get_host_port(&redis).unwrap();
    let connection_string = set_redis_connection_environment_variable(host_redis_port);
    let manifest_name = "test";
    let manifest_reference = "exists";
    let manifest = generate_manifest_body(DEFAULT_DIGEST);
    add_manifest(
        manifest_name,
        manifest_reference,
        &manifest,
        connection_string,
    );
    let client = Client::tracked(rocket())
        .await
        .expect("valid rocket instance");
    let uri = format!("/v2/{}/manifests/{}", manifest_name, DEFAULT_DIGEST);
    let response = client.delete(uri).dispatch().await;
    assert_eq!(response.status(), Status::Accepted);
}

// #[tokio::test]
// async fn blob_can_be_downloaded() {
//     let docker_client = docker_client();
//     let redis = run_redis(&docker_client).await;
//     let host_redis_port = get_host_port(&redis).unwrap();
//     let connection_string = set_redis_connection_environment_variable(host_redis_port);
//     let manifest_name = "test";
//     let manifest_reference = "exists";
//     let manifest = generate_manifest_body(DEFAULT_DIGEST);
//     add_manifest(manifest_name, manifest_reference, &manifest, connection_string);
//     let dir_path = tempdir().expect("temporary folder").path();
//     add_blob(manifest, dir_path);
//     let client = Client::tracked(rocket()).await.expect("valid rocket instance");
//     let uri = format!("/v2/{}/blobs/{}", manifest_name, DEFAULT_DIGEST);
//     let response = client.get(uri).dispatch().await;
//     assert_eq!(response.status(), Status::Ok);
//     //assert_eq!(response.headers().get("Docker-Content-Digest"), DEFAULT_DIGEST);
// }

fn docker_client() -> Cli {
    clients::Cli::default()
}

async fn run_redis<'redis>(docker_client: &'_ Cli) -> Container<'_, Cli, RedisImage> {
    let redis_node: Container<'_, Cli, RedisImage> = docker_client.run_with_args(
        redis_image::Redis::default().with_tag("6.2-alpine"),
        RunArgs::default().with_mapped_port((portpicker::pick_unused_port().unwrap(), REDIS_PORT)),
    );
    redis_node
}

fn get_host_port(redis_container: &Container<Cli, RedisImage>) -> Option<u16> {
    redis_container.get_host_port(REDIS_PORT)
}

fn set_redis_connection_environment_variable(port: u16) -> String {
    let connection_string = format_redis_connection_string(port);
    env::set_var(REDIS_CONNECTION_ENV, connection_string.clone());
    connection_string
}

fn format_redis_connection_string(port: u16) -> String {
    format!("redis://localhost:{}/", port)
}

fn add_manifest(name: &str, reference: &str, value: &Manifest, connection_string: String) {
    let key = format!("manifest::{}::{}", name, reference);
    let alias_key = format!("manifest::{}::{}::alias", name, value.config.digest);
    let mut connection = redis_client::open(connection_string)
        .unwrap()
        .get_connection()
        .unwrap();
    let manifest = connection.set::<String, &Manifest, bool>(key, value);
    let alias = connection.sadd::<String, String, bool>(alias_key, reference.to_string());
    match manifest {
        Ok(_) => match alias {
            Ok(_) => println!("Ok!"),
            Err(err) => println!("{}", err),
        },
        Err(err) => println!("{}", err),
    }
}

fn generate_manifest_body(digest: &str) -> Manifest {
    Manifest {
        schema_version: 1,
        media_type: "random media type".to_string(),
        config: Descriptor {
            media_type: "application/vnd.oci.image.config.v1+json".to_string(),
            digest: digest.to_string(),
            size: 0,
            urls: vec!["http://random1".to_string(), "https://random2".to_string()],
            annotations: Default::default(),
        },
        layers: vec![Descriptor {
            media_type: "application/vnd.oci.image.layer.v1.tar".to_string(),
            digest: "random digest".to_string(),
            size: 0,
            urls: vec![],
            annotations: Default::default(),
        }],
        annotations: Default::default(),
    }
}
