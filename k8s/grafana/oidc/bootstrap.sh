#!/usr/bin/env bash
set -euo pipefail

KEYCLOAK_NAMESPACE="${KEYCLOAK_NAMESPACE:-keycloak}"
KEYCLOAK_RELEASE="${KEYCLOAK_RELEASE:-keycloak}"
KEYCLOAK_REALM="${KEYCLOAK_REALM:-c2}"
KEYCLOAK_URL="${KEYCLOAK_URL:-https://keycloak.c2.local}"

GRAFANA_URL="${GRAFANA_URL:-https://grafana.c2.local}"
GRAFANA_CLIENT_ID="${GRAFANA_CLIENT_ID:-grafana}"

ADMIN_SECRET="${KEYCLOAK_ADMIN_SECRET:-${KEYCLOAK_RELEASE}-admin}"
ADMIN_USER="${KEYCLOAK_ADMIN_USER:-$(kubectl -n "${KEYCLOAK_NAMESPACE}" get secret "${ADMIN_SECRET}" -o jsonpath='{.data.admin-user}' | base64 -d)}"
ADMIN_PASSWORD="${KEYCLOAK_ADMIN_PASSWORD:-$(kubectl -n "${KEYCLOAK_NAMESPACE}" get secret "${ADMIN_SECRET}" -o jsonpath='{.data.admin-password}' | base64 -d)}"

KC_ADMIN=(kubectl -n "${KEYCLOAK_NAMESPACE}" exec deploy/"${KEYCLOAK_RELEASE}" -- /opt/keycloak/bin/kcadm.sh)

"${KC_ADMIN[@]}" config credentials \
  --server http://localhost:8080 \
  --realm master \
  --user "${ADMIN_USER}" \
  --password "${ADMIN_PASSWORD}"

redirect_uri="${GRAFANA_URL}/login/generic_oauth"

client_id=$("${KC_ADMIN[@]}" get clients -r "${KEYCLOAK_REALM}" -q clientId="${GRAFANA_CLIENT_ID}" | sed -n 's/.*"id" : "\([^"]*\)".*/\1/p' | head -n1)
if [[ -z "${client_id}" ]]; then
  "${KC_ADMIN[@]}" create clients -r "${KEYCLOAK_REALM}" \
    -s clientId="${GRAFANA_CLIENT_ID}" \
    -s enabled=true \
    -s protocol=openid-connect \
    -s publicClient=false \
    -s standardFlowEnabled=true \
    -s directAccessGrantsEnabled=false \
    -s serviceAccountsEnabled=false \
    -s "redirectUris=[\"${redirect_uri}\"]" \
    -s "webOrigins=[\"${GRAFANA_URL}\"]" \
    -s rootUrl="${GRAFANA_URL}" \
    -s baseUrl="${GRAFANA_URL}" \
    -s adminUrl="${GRAFANA_URL}"

  client_id=$("${KC_ADMIN[@]}" get clients -r "${KEYCLOAK_REALM}" -q clientId="${GRAFANA_CLIENT_ID}" | sed -n 's/.*"id" : "\([^"]*\)".*/\1/p' | head -n1)
else
  "${KC_ADMIN[@]}" update clients/"${client_id}" -r "${KEYCLOAK_REALM}" \
    -s "redirectUris=[\"${redirect_uri}\"]" \
    -s "webOrigins=[\"${GRAFANA_URL}\"]" \
    -s rootUrl="${GRAFANA_URL}" \
    -s baseUrl="${GRAFANA_URL}" \
    -s adminUrl="${GRAFANA_URL}"
fi

mapper_name="groups"
mapper_lookup=$("${KC_ADMIN[@]}" get clients/"${client_id}"/protocol-mappers/models -r "${KEYCLOAK_REALM}" | grep -q '"name" : "'"${mapper_name}"'"' && echo "present" || true)
if [[ -z "${mapper_lookup}" ]]; then
  "${KC_ADMIN[@]}" create clients/"${client_id}"/protocol-mappers/models -r "${KEYCLOAK_REALM}" \
    -s name="${mapper_name}" \
    -s protocol=openid-connect \
    -s protocolMapper=oidc-group-membership-mapper \
    -s 'config={"full.path":"true","id.token.claim":"true","access.token.claim":"true","userinfo.token.claim":"true","claim.name":"groups"}'
fi

client_secret=$("${KC_ADMIN[@]}" get clients/"${client_id}"/client-secret -r "${KEYCLOAK_REALM}" | sed -n 's/.*"value" : "\([^"]*\)".*/\1/p' | head -n1)

cat <<INFO
Grafana OIDC settings
---------------------
OIDC Provider Name: Keycloak
OIDC Issuer: ${KEYCLOAK_URL}/realms/${KEYCLOAK_REALM}
OIDC Client ID: ${GRAFANA_CLIENT_ID}
OIDC Client Secret: ${client_secret}
Redirect URI: ${redirect_uri}
INFO
