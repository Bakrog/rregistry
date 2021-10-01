use super::tags::{is_accepted_digest, is_tag_name_valid};
use super::Descriptor;

use anyhow::{bail, Error, Result};

use r2d2::{Pool, PooledConnection};

use redis::{
    Client, Commands, ConnectionLike, ErrorKind, FromRedisValue, RedisError, RedisResult,
    RedisWrite, ToRedisArgs, Value,
};
use regex::Regex;

use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::{delete, get, head, State};

use std::collections::HashMap;
use std::ops::Add;

/// Prefix for storing manifest at Redis
const MANIFEST_PREFIX_KEY: &str = "manifest";
/// Suffix for stored alias at Redis
const MANIFEST_ALIAS_SUFFIX_KEY: &str = "alias";

/// Represents an [OCI Image manifest](https://github.com/opencontainers/image-spec/blob/main/manifest.md)
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "rocket::serde", rename_all = "camelCase")]
pub struct Manifest {
    /// This REQUIRED property specifies the image manifest schema version.
    /// For this version of the specification, this MUST be `2` to ensure backward
    /// compatibility with older versions of Docker. The value of this field
    /// will not change. This field MAY be removed in a future version of the specification.
    pub schema_version: usize,
    /// This property is reserved for use, to [maintain compatibility](https://github.com/opencontainers/image-spec/blob/main/media-types.md#compatibility-matrix).
    /// When used, this field contains the media type of this document, which differs
    /// from the [descriptor](https://github.com/opencontainers/image-spec/blob/main/descriptor.md#properties)
    /// use of `mediaType`.
    pub media_type: String,
    /// This REQUIRED property references a configuration object for a container, by digest.
    pub config: Descriptor,
    /// Each item in the array MUST be a [descriptor](https://github.com/opencontainers/image-spec/blob/main/descriptor.md).
    /// The array MUST have the base layer at index `0`. Subsequent layers MUST then
    /// follow in stack order (i.e. from `layers[0]` to `layers[len(layers)-1]`).
    /// The final filesystem layout MUST match the result of [applying](https://github.com/opencontainers/image-spec/blob/main/layer.md#applying-changesets)
    /// the layers to an empty directory. The [ownership, mode, and other attributes](https://github.com/opencontainers/image-spec/blob/main/layer.md#file-attributes)
    /// of the initial empty directory are unspecified.
    pub layers: Vec<Descriptor>,
    /// This OPTIONAL property contains arbitrary metadata for the image manifest.
    /// This OPTIONAL property MUST use the [annotation rules](https://github.com/opencontainers/image-spec/blob/main/annotations.md#rules).
    ///
    /// See [Pre-Defined Annotation Keys](https://github.com/opencontainers/image-spec/blob/main/annotations.md#pre-defined-annotation-keys).
    pub annotations: HashMap<String, String>,
}

/// Deserialize the manifest binary from redis to an Object
impl FromRedisValue for Manifest {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        match *v {
            Value::Data(ref bytes) => Ok(bincode::deserialize(bytes).unwrap()),
            Value::Nil => Err(RedisError::from((
                ErrorKind::IoError,
                "Couldn't find manifest",
            ))),
            _ => panic!("Response type not string compatible."),
        }
    }
}

/// Serialize a manifest object to binary
impl ToRedisArgs for Manifest {
    fn write_redis_args<W>(&self, vec: &mut W)
    where
        W: ?Sized + RedisWrite,
    {
        let bytes = bincode::serialize(self).unwrap();
        vec.write_arg(bytes.as_slice())
    }
}

/// Check if the manifest exists
#[head("/<name>/manifests/<reference>")]
pub async fn check_manifest(
    name: &str,
    reference: &str,
    connection_pool: &State<Pool<Client>>,
) -> Status {
    if !is_valid_request(name, reference) {
        return Status::NotFound;
    }
    let mut con = connection_pool.get().unwrap();
    let exists_key = manifest_exist(name, reference, &mut con).expect("couldn't find keys");
    if exists_key {
        Status::Ok
    } else {
        Status::NotFound
    }
}

/// Get a manifest using:
/// - `name`: The manifest name
/// - `reference`: The manifest tag or digest
#[get("/<name>/manifests/<reference>")]
pub async fn get_manifest(
    name: &str,
    reference: &str,
    connection_pool: &State<Pool<Client>>,
) -> Option<Json<Manifest>> {
    if !is_valid_request(name, reference) {
        return None;
    }
    let mut con = connection_pool
        .get()
        .expect("couldn't get connection to redis");
    let manifest = manifest(name, reference, &mut con).expect("couldn't find manifest");
    Some(Json(manifest))
}

/// Delete a manifest using:
/// - `name`: The manifest name
/// - `reference`: The manifest tag or digest
///
/// Deleting a manifest digest means that all tags will be deleted.
#[delete("/<name>/manifests/<reference>")]
pub async fn delete_manifest(
    name: &str,
    reference: &str,
    connection_pool: &State<Pool<Client>>,
) -> Status {
    if !is_valid_request(name, reference) {
        return Status::NotFound;
    }
    let mut con = connection_pool
        .get()
        .expect("couldn't get connection to redis");
    match delete(name, reference, &mut con) {
        Ok(removed_manifests) => {
            if removed_manifests > 0 {
                Status::Accepted
            } else {
                Status::NotFound
            }
        }
        Err(_) => Status::NotFound,
    }
}

#[doc(hidden)]
fn is_valid_request(name: &str, reference: &str) -> bool {
    is_manifest_name_valid(name) && (is_tag_name_valid(reference) || is_accepted_digest(reference))
}

/// Verify if the manifest name is valid using the regex
/// `^[a-z0-9]+([._-][a-z0-9]+)*(/[a-z0-9]+([._-][a-z0-9]+)*)*$`
pub fn is_manifest_name_valid(name: &str) -> bool {
    let regex = Regex::new(r"^[a-z0-9]+([._-][a-z0-9]+)*(/[a-z0-9]+([._-][a-z0-9]+)*)*$").unwrap();
    regex.is_match(name)
}

#[doc(hidden)]
fn generate_manifest_key<'manifest>(name: &'manifest str, reference: &'manifest str) -> String {
    format!("{}::{}::{}", MANIFEST_PREFIX_KEY, name, reference)
}

#[doc(hidden)]
fn generate_alias_key<'manifest>(name: &'manifest str, digest: &'manifest str) -> String {
    format!(
        "{}::{}::{}::{}",
        MANIFEST_PREFIX_KEY, name, digest, MANIFEST_ALIAS_SUFFIX_KEY
    )
}

/// Search at redis if an manifest exists
fn manifest_exist(name: &str, reference: &str, con: &mut PooledConnection<Client>) -> Result<bool> {
    let key = &generate_manifest_key(name, reference);
    let alias_key = &generate_alias_key(name, reference);
    let exists_key = con.exists(key).expect("connection with redis");
    let exists_alias = con.exists(alias_key).expect("connection with redis");
    Ok(exists_key || exists_alias)
}

/// Retrieves a manifest from redis
fn manifest(name: &str, reference: &str, con: &mut PooledConnection<Client>) -> Result<Manifest> {
    let key = generate_manifest_key(name, reference);
    match con.get(key) {
        Ok(manifest) => Ok(manifest),
        Err(_) => {
            let alias_key = &generate_alias_key(name, reference);
            let search_alias: RedisResult<Vec<String>> = con.smembers(alias_key);
            match search_alias {
                Ok(alias) => {
                    let existing_alias = alias
                        .iter()
                        .filter(|alias_key| manifest_exist(name, alias_key, con).unwrap())
                        .take(1)
                        .last()
                        .expect("couldn't find an existing alias");
                    manifest(name, existing_alias, con)
                }
                Err(_) => bail!("Couldn't find manifest"),
            }
        }
    }
}

/// Delete a manifest
fn delete(name: &str, reference: &str, con: &mut PooledConnection<Client>) -> Result<i8> {
    let key = generate_manifest_key(name, reference);
    let result = con
        .req_command(redis::cmd("GETDEL").arg(key))
        .map(|value| Manifest::from_redis_value(&value))
        .unwrap();
    match result {
        Ok(manifest) => {
            let sum = if is_accepted_digest(reference) {
                search_alias_and_delete_it(&name, reference, con)
            } else {
                remove_tag_relation_from_digest(name, reference, con, manifest)
            }
            .expect(format!("couldn't delete all elements of {}/{}", name, reference).as_str());
            Ok(sum)
        }
        Err(_) => search_alias_and_delete_it(&name, &reference, con),
    }
}

/// Search manifest by alias and delete it
fn search_alias_and_delete_it(
    name: &str,
    reference: &str,
    con: &mut PooledConnection<Client>,
) -> Result<i8, Error> {
    let alias_key = &generate_alias_key(name, reference);
    let search_alias: RedisResult<Vec<String>> = con.smembers(alias_key);
    match search_alias {
        Ok(alias) => {
            let sum = delete_alias(name, con, alias)
                .unwrap()
                .add(delete_alias_key(con, alias_key).unwrap());
            Ok(sum)
        }
        Err(_) => Ok(0),
    }
}

/// Delete alias
fn delete_alias(name: &str, con: &mut PooledConnection<Client>, alias: Vec<String>) -> Result<i8> {
    let mut sum: i8 = 0;
    alias.iter().for_each(|alias_key| {
        let key_to_be_deleted = generate_manifest_key(name, alias_key);
        sum += con.del::<String, i8>(key_to_be_deleted).unwrap();
    });
    Ok(sum)
}

/// Delete alias key
fn delete_alias_key(con: &mut PooledConnection<Client>, alias_key: &String) -> Result<i8> {
    Ok(con.del::<String, i8>(alias_key.clone()).unwrap())
}

/// Remove tag from digest
fn remove_tag_relation_from_digest(
    name: &str,
    reference: &str,
    con: &mut PooledConnection<Client>,
    manifest: Manifest,
) -> Result<i8, Error> {
    let alias_key = generate_alias_key(name, manifest.config.digest.as_str());
    let response = con.srem(alias_key, reference).unwrap();
    Ok(response)
}
