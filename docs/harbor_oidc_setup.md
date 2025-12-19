Harbor OIDC fields

- OIDC Provider Name: Keycloak
- OIDC Endpoint: https://keycloak.c2.local/realms/c2
- OIDC Client ID: harbor
- OIDC Client Secret: Tzwxi5bndTls5tGAbhcE7Zfak93LWJAJ
- OIDC Scope: openid,profile,email
- Group Claim Name: groups
- OIDC Admin Group: /c2-super-admins
- OIDC Group Filter: ^/c2-(super-admins|devsecops|developers|clients-readonly|field-operators)$
- Username Claim: preferred_username
- Verify Certificate: unchecked (self-signed dev TLS)
- Automatic onboarding: checked (recommended)
- OIDC Session Logout: optional (I’d enable it)

Redirect URI is already configured in Keycloak:

- https://harbor.c2.local/c/oidc/callback
