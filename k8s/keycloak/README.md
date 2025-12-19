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

## LDAP Setup

Use the Keycloak admin console to add an LDAP user federation provider.
Configure the bind DN, bind credential, base DN, and sync settings for your directory.
