# Docker Build Guide

Use the multi-service Dockerfile to build any C2 binary.

## Build

```sh
docker build -f docker/Dockerfile --build-arg BIN=c2-api -t c2-api:local .
docker build -f docker/Dockerfile --build-arg BIN=c2-gateway -t c2-gateway:local .
docker build -f docker/Dockerfile --build-arg BIN=c2-web -t c2-web:local .
docker build -f docker/Dockerfile --build-arg BIN=c2-mcp -t c2-mcp:local .
docker build -f docker/Dockerfile --build-arg BIN=c2-worker -t c2-worker:local .
docker build -f docker/Dockerfile --build-arg BIN=c2-operator -t c2-operator:local .
```

## Run

```sh
docker run --rm -p 8080:8080 c2-api:local
```

Set runtime configuration via environment variables (see `docs/LOCAL_SETUP.md`).
