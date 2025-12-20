# Grafana OIDC (Keycloak)

This config wires Grafana to Keycloak via OIDC so Grafana users authenticate
with the `c2` realm.

## Bootstrap Keycloak client

```sh
./k8s/grafana/oidc/bootstrap.sh
```

Create the Grafana OIDC secret in the cluster (do not commit secrets to Git). You
can use `k8s/overlays/dev/grafana-oidc.env.example` as a local template:

```sh
cp k8s/overlays/dev/grafana-oidc.env.example k8s/overlays/dev/grafana-oidc.env
# edit and set GRAFANA_OIDC_CLIENT_SECRET=...

kubectl -n c2-system create secret generic c2-grafana-oidc \
  --from-env-file=k8s/overlays/dev/grafana-oidc.env \
  --dry-run=client -o yaml | kubectl apply -f -
```

## Redirect URI

The redirect URI must be `https://grafana.c2.local/login/generic_oauth`.
