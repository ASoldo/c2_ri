# Kubernetes Deployments

This directory provides a Kustomize layout for C2 deployments.

## Overlays

- `k8s/overlays/dev` (minikube/local)
- `k8s/overlays/staging`
- `k8s/overlays/prod`

Apply an overlay:

```sh
kubectl apply -k k8s/overlays/dev
```

## Minikube Notes

Install Harbor for local image hosting (see `k8s/harbor/README.md`), then build and push images:

```sh
docker build -f docker/Dockerfile --build-arg BIN=c2-operator -t harbor.c2.local/c2/c2-operator:dev .
docker build -f docker/Dockerfile --build-arg BIN=c2-api -t harbor.c2.local/c2/c2-api:dev .
docker build -f docker/Dockerfile --build-arg BIN=c2-gateway -t harbor.c2.local/c2/c2-gateway:dev .
docker build -f docker/Dockerfile --build-arg BIN=c2-web -t harbor.c2.local/c2/c2-web:dev .
docker build -f docker/Dockerfile --build-arg BIN=c2-mcp -t harbor.c2.local/c2/c2-mcp:dev .
docker build -f docker/Dockerfile --build-arg BIN=c2-worker -t harbor.c2.local/c2/c2-worker:dev .

docker push harbor.c2.local/c2/c2-operator:dev
docker push harbor.c2.local/c2/c2-api:dev
docker push harbor.c2.local/c2/c2-gateway:dev
docker push harbor.c2.local/c2/c2-web:dev
docker push harbor.c2.local/c2/c2-mcp:dev
docker push harbor.c2.local/c2/c2-worker:dev
```

The dev overlay expects a Harbor project named `c2` and uses
`harbor.c2.local/c2` as the image registry prefix.

Ingress hosts for dev overlay:

- `c2.local`
- `grafana.c2.local`
- `prometheus.c2.local`
- `harbor.c2.local`
- `keycloak.c2.local`

Map these to the minikube IP in `/etc/hosts`.

Example:

```text
192.168.49.2 c2.local grafana.c2.local prometheus.c2.local harbor.c2.local keycloak.c2.local
```

Keycloak runs in a dedicated `keycloak` namespace. Install it using
`k8s/keycloak/README.md`.

## Secrets

Create the Harbor registry pull secret and SurrealDB secret before applying overlays:

```sh
kubectl -n c2-system create secret docker-registry harbor-registry \
  --docker-server=harbor.c2.local \
  --docker-username=admin \
  --docker-password=CHANGE_ME \
  --docker-email=admin@c2.local
```

If you use the self-signed Harbor TLS cert, make sure Docker trusts it or mark
`harbor.c2.local` as an insecure registry before pushing/pulling images.

Create the SurrealDB secret:

```sh
kubectl -n c2-system create secret generic c2-surreal-credentials --from-literal=password=CHANGEME
```

The dev overlay includes a SurrealDB deployment suitable for local testing.
Update Grafana admin credentials in `k8s/base/observability/grafana.yaml` before production use.
