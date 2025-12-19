# Deployment

This document covers container builds and Kubernetes operator deployment.

## Docker

Build a service image using the shared Dockerfile:

```sh
docker build -f docker/Dockerfile --build-arg BIN=c2-api -t c2-api:local .
```

## Kubernetes Operator

1. Apply the CRD (regenerate after spec changes if needed):

```sh
kubectl apply -f k8s/base/crd/c2clusters.yaml
```

Optional CRD regeneration:

```sh
cargo run -p c2-operator --bin crdgen > k8s/base/crd/c2clusters.yaml
```

2. Deploy the operator:

```sh
kubectl apply -f k8s/base/operator.yaml
```

3. Create a C2 cluster resource:

```sh
kubectl apply -f k8s/base/c2cluster.yaml
```

Update the `spec.image` fields to reference your registry and version. Set `spec.runtime.metricsPort`
to expose Prometheus scrape endpoints for the services (use a port distinct from the service ports).

## Observability Stack

Deploy Prometheus and Grafana:

```sh
kubectl apply -f k8s/base/observability/prometheus.yaml
kubectl apply -f k8s/base/observability/grafana.yaml
```

Update the Grafana admin secret in `k8s/base/observability/grafana.yaml` before production use.

## Kustomize (Recommended)

Use the environment overlays under `k8s/`:

```sh
kubectl apply -k k8s/overlays/dev
```

See `k8s/README.md` for minikube and image loading steps.
For minikube Harbor registry setup, see `k8s/harbor/README.md`.

Keycloak for LDAP-backed auth can be installed with Helm; see `k8s/keycloak/README.md`.

## ArgoCD

Use the Application manifests under `k8s/argocd` after installing ArgoCD.
