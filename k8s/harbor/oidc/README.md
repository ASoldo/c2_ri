# Harbor OIDC (Keycloak)

This config wires Harbor to Keycloak via OIDC so Harbor users authenticate with the `c2` realm.

## Bootstrap Keycloak client

```sh
./k8s/harbor/oidc/bootstrap.sh
```

The script prints the exact fields to copy into Harbor's **Administration → Configuration → Authentication → OIDC** page.

## Harbor UI fields

Use the output from the script. Defaults are:

- OIDC Provider Name: `Keycloak`
- OIDC Endpoint: `https://keycloak.c2.local/realms/c2`
- OIDC Client ID: `harbor`
- OIDC Client Secret: (from script)
- OIDC Scope: `openid,profile,email`
- Group Claim Name: `groups`
- OIDC Admin Group: `/c2-super-admins`
- OIDC Group Filter: `^/c2-(super-admins|devsecops|developers|clients-readonly|field-operators)$`
- Username Claim: `preferred_username`
- Verify Certificate: disable for self-signed dev TLS
- Automatic onboarding: enable for first-login provisioning

The redirect URI must be `https://harbor.c2.local/c/oidc/callback`.

Harbor enforces HTTPS for the OIDC endpoint, so Keycloak ingress must have TLS
enabled even in dev. See `k8s/keycloak/README.md` for the self-signed setup.
