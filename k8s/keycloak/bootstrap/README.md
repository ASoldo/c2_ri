# Keycloak bootstrap (C2 roles/users)

This script creates the `c2` realm, roles, groups, and the initial super-admin user.
It is safe to re-run; existing items are left in place.

## Run

```sh
KEYCLOAK_NAMESPACE=keycloak \
KEYCLOAK_RELEASE=keycloak \
SUPERADMIN_PASSWORD='CHANGE_ME' \
SUPERADMIN_TEMP_PASSWORD=true \
./k8s/keycloak/bootstrap/bootstrap.sh
```

## Defaults

- Realm: `c2`
- Super-admin user: `andrej.soldo` (Andrej Soldo)
- Roles:
  - `c2_super_admin`
  - `c2_devsecops`
  - `c2_developer`
  - `c2_client_readonly`
  - `c2_field_operator`
- Groups:
  - `c2-super-admins`
  - `c2-devsecops`
  - `c2-developers`
  - `c2-clients-readonly`
  - `c2-field-operators`

Super-admins receive the Keycloak `realm-admin` role. DevSecOps receives
`view-realm`, `view-users`, and `manage-users` for the `realm-management` client.

Override any value via environment variables in `k8s/keycloak/bootstrap/bootstrap.sh`.
