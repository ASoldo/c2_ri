# Harbor Registry (Minikube)

This setup uses the official Harbor Helm chart with a NodePort for local access.

## Install Harbor

```sh
helm repo add harbor https://helm.goharbor.io
helm repo update
kubectl create namespace harbor
helm upgrade --install harbor harbor/harbor \
  --namespace harbor \
  -f k8s/harbor/values-dev.yaml
```

## Access

- UI: `http://harbor.c2.local:32080`
- Default user: `admin`
- Password: set in `k8s/harbor/values-dev.yaml`

Make sure `harbor.c2.local` resolves to your minikube IP.

## Docker Login

```sh
docker login harbor.c2.local:32080
```

Create a `c2` project in Harbor before pushing images.

If Docker blocks HTTP registries, add `harbor.c2.local:32080` to the daemon's
insecure registries list and restart Docker.
