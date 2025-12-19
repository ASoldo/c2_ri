# Keycloak (Minikube)

Install Keycloak with the local Helm chart for dev testing.

```sh
kubectl create namespace keycloak
helm dependency update k8s/charts/keycloak
helm upgrade --install keycloak k8s/charts/keycloak \
  --namespace keycloak \
  -f k8s/keycloak/values-dev.yaml
```

Access: `http://keycloak.c2.local` (redirects to the `c2` realm account console).
Admin console: `http://keycloak.c2.local/admin/c2/console`

Admin credentials are defined in `k8s/keycloak/values-dev.yaml`.

## Bootstrap roles/users

```sh
SUPERADMIN_PASSWORD='CHANGE_ME' \
SUPERADMIN_TEMP_PASSWORD=true \
./k8s/keycloak/bootstrap/bootstrap.sh
```

Usernames follow the `first.last` format (lowercase). Emails remain unchanged.

## TLS (required for Harbor OIDC)

Harbor requires an HTTPS OIDC endpoint. For dev, create a self-signed cert and
load it into the Keycloak ingress:

```sh
openssl req -x509 -nodes -newkey rsa:2048 -days 3650 \
  -keyout /tmp/keycloak.c2.local.key \
  -out /tmp/keycloak.c2.local.crt \
  -subj "/CN=keycloak.c2.local" \
  -addext "subjectAltName=DNS:keycloak.c2.local"

kubectl -n keycloak create secret tls keycloak-tls \
  --cert=/tmp/keycloak.c2.local.crt \
  --key=/tmp/keycloak.c2.local.key \
  --dry-run=client -o yaml | kubectl apply -f -

helm upgrade --install keycloak k8s/charts/keycloak \
  --namespace keycloak \
  -f k8s/keycloak/values-dev.yaml
```

## LDAP Setup

Use the Keycloak admin console to add an LDAP user federation provider.
Configure the bind DN, bind credential, base DN, and sync settings for your directory.
