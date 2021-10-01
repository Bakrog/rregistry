# rregistry

## RRegistry

`rregistry` is a container registry wrote with rust programming language.

**NB! Currently, it's a work in progress.**

## How to use

To use it you need [redis](https://redis.io) (used to search for container
manifests) and the following environment variables:
- REDIS_CONNECTION_STRING: Connection string to redis, e.g. `redis://localhost:6379`
- STORAGE_PATH: Path to store container layers, normally tar or tar.gz files

## Roadmap
- [x] Add ability to download manifests
- [ ] Add ability to download layers
- [ ] Add manifest through rest endpoint
- [ ] Add layer through rest endpoint
- [ ] Add layer redirecting to another service
- [ ] Clone manifest from another repository
- [ ] Clone layers from another repository
- [ ] Implement media type restrictions

## Useful links
- [Open container distribution specification](https://github.com/opencontainers/distribution-spec/blob/main/spec.md#pull)
- [Registry conformance tooling](https://github.com/opencontainers/distribution-spec/tree/main/conformance)

# License
[Apache-2.0](LICENSE)
