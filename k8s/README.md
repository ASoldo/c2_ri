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

Build images for the cluster and load them into minikube:

```sh
docker build -f docker/Dockerfile --build-arg BIN=c2-operator -t c2-operator:dev .
docker build -f docker/Dockerfile --build-arg BIN=c2-api -t c2-api:dev .
docker build -f docker/Dockerfile --build-arg BIN=c2-gateway -t c2-gateway:dev .
docker build -f docker/Dockerfile --build-arg BIN=c2-web -t c2-web:dev .
docker build -f docker/Dockerfile --build-arg BIN=c2-mcp -t c2-mcp:dev .
docker build -f docker/Dockerfile --build-arg BIN=c2-worker -t c2-worker:dev .

minikube image load c2-operator:dev
minikube image load c2-api:dev
minikube image load c2-gateway:dev
minikube image load c2-web:dev
minikube image load c2-mcp:dev
minikube image load c2-worker:dev
```

Ingress hosts for dev overlay:

- `c2.local`
- `grafana.c2.local`
- `prometheus.c2.local`

Map these to the minikube IP in `/etc/hosts`.

## Secrets

Create the SurrealDB secret before applying overlays:

```sh
kubectl -n c2-system create secret generic c2-surreal-credentials --from-literal=password=CHANGEME
```

Update Grafana admin credentials in `k8s/base/observability/grafana.yaml` before production use.
