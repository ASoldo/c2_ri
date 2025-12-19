# ArgoCD Setup

1. Install ArgoCD in your cluster:

```sh
kubectl create namespace argocd

# Create a dev TLS secret for Argo CD (self-signed)
openssl req -x509 -nodes -newkey rsa:4096 -days 365 \
  -keyout /tmp/argocd.c2.local.key \
  -out /tmp/argocd.c2.local.crt \
  -subj "/CN=argocd.c2.local" \
  -addext "subjectAltName=DNS:argocd.c2.local"
kubectl -n argocd create secret tls argocd-tls \
  --cert=/tmp/argocd.c2.local.crt \
  --key=/tmp/argocd.c2.local.key

# Configure Keycloak OIDC and create the client secret
./k8s/argocd/oidc/bootstrap.sh
cp k8s/argocd/secret.env.example k8s/argocd/secret.env
# edit and set oidc.keycloak.clientSecret=...

# Install Argo CD + ingress + OIDC config
kubectl apply -k k8s/argocd
```

2. Apply the ArgoCD project + application:

```sh
kubectl apply -f k8s/argocd/project.yaml
kubectl apply -f k8s/argocd/c2-dev.yaml
```

Switch to staging/prod by applying the appropriate application manifest.

## Access

- UI: `https://argocd.c2.local`

Log in with Keycloak and use your group-based role. Admin users are mapped via
`k8s/argocd/argocd-rbac-cm.yaml`.

For self-signed Keycloak TLS in dev, Argo CD is configured with
`oidc.tls.insecure.skip.verify: "true"`. Remove this and use a trusted CA
(`rootCA` in `oidc.config`) for staging/prod.
