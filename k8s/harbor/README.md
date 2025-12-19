# Harbor Registry (Minikube)

This setup uses the official Harbor Helm chart with HTTPS via ingress.

## Install Harbor

```sh
helm repo add harbor https://helm.goharbor.io
helm repo update
kubectl create namespace harbor

# Create a dev TLS secret for Harbor (self-signed)
openssl req -x509 -nodes -newkey rsa:4096 -days 365 \
  -keyout /tmp/harbor.c2.local.key \
  -out /tmp/harbor.c2.local.crt \
  -subj "/CN=harbor.c2.local" \
  -addext "subjectAltName=DNS:harbor.c2.local"
kubectl -n harbor create secret tls harbor-tls \
  --cert=/tmp/harbor.c2.local.crt \
  --key=/tmp/harbor.c2.local.key

# If you need to update the secret later:
# kubectl -n harbor delete secret harbor-tls
helm upgrade --install harbor harbor/harbor \
  --namespace harbor \
  -f k8s/harbor/values-dev.yaml
```

## Access

- UI: `https://harbor.c2.local`
- Default user: `admin`
- Password: set in `k8s/harbor/values-dev.yaml`

Make sure `harbor.c2.local` resolves to your minikube IP.

## Docker Login

```sh
docker login harbor.c2.local
```

Create a `c2` project in Harbor before pushing images.

If Docker blocks the self-signed cert, either trust the CA or mark the registry
as insecure.

Trust the self-signed cert:

```sh
sudo mkdir -p /etc/docker/certs.d/harbor.c2.local
kubectl -n harbor get secret harbor-tls -o jsonpath='{.data.tls\.crt}' | base64 -d | sudo tee /etc/docker/certs.d/harbor.c2.local/ca.crt
sudo systemctl restart docker
```

Minikube (docker runtime) also needs the CA to pull images:

```sh
kubectl -n harbor get secret harbor-tls -o jsonpath='{.data.tls\.crt}' | base64 -d > /tmp/harbor.c2.local.crt
minikube cp /tmp/harbor.c2.local.crt /tmp/harbor.c2.local.crt
minikube ssh -- 'sudo mkdir -p /etc/docker/certs.d/harbor.c2.local && sudo cp /tmp/harbor.c2.local.crt /etc/docker/certs.d/harbor.c2.local/ca.crt'
```

Or add `harbor.c2.local` to Docker's `insecure-registries` list and restart Docker.

## OIDC (Keycloak)

Harbor should authenticate against Keycloak via OIDC (not LDAP).
See `k8s/harbor/oidc/README.md` for the bootstrap script and UI fields.
