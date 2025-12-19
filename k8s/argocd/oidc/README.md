# Argo CD OIDC (Keycloak)

This config wires Argo CD to Keycloak via OIDC so Argo CD users authenticate
with the `c2` realm.

## Bootstrap Keycloak client

```sh
./k8s/argocd/oidc/bootstrap.sh
```

Copy the client secret into `k8s/argocd/secret.env`:

```sh
cp k8s/argocd/secret.env.example k8s/argocd/secret.env
# edit and set oidc.keycloak.clientSecret=...
```

## Redirect URI

The redirect URI must be `https://argocd.c2.local/auth/callback`.
