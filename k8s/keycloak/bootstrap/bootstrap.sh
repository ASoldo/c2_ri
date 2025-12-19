#!/usr/bin/env bash
set -euo pipefail

KEYCLOAK_NAMESPACE="${KEYCLOAK_NAMESPACE:-keycloak}"
KEYCLOAK_RELEASE="${KEYCLOAK_RELEASE:-keycloak}"
KEYCLOAK_REALM="${KEYCLOAK_REALM:-c2}"

ADMIN_SECRET="${KEYCLOAK_ADMIN_SECRET:-${KEYCLOAK_RELEASE}-admin}"

ADMIN_USER="${KEYCLOAK_ADMIN_USER:-$(kubectl -n "${KEYCLOAK_NAMESPACE}" get secret "${ADMIN_SECRET}" -o jsonpath='{.data.admin-user}' | base64 -d)}"
ADMIN_PASSWORD="${KEYCLOAK_ADMIN_PASSWORD:-$(kubectl -n "${KEYCLOAK_NAMESPACE}" get secret "${ADMIN_SECRET}" -o jsonpath='{.data.admin-password}' | base64 -d)}"

SUPERADMIN_FIRSTNAME="${SUPERADMIN_FIRSTNAME:-Andrej}"
SUPERADMIN_LASTNAME="${SUPERADMIN_LASTNAME:-Soldo}"
default_username="$(printf '%s.%s' "${SUPERADMIN_FIRSTNAME}" "${SUPERADMIN_LASTNAME}" | tr '[:upper:]' '[:lower:]' | tr ' ' '.' | tr -s '.')"
SUPERADMIN_USERNAME="${SUPERADMIN_USERNAME:-${default_username}}"
SUPERADMIN_EMAIL="${SUPERADMIN_EMAIL:-soldo.andrej@gmail.com}"
SUPERADMIN_PASSWORD="${SUPERADMIN_PASSWORD:-change-me}"
SUPERADMIN_TEMP_PASSWORD="${SUPERADMIN_TEMP_PASSWORD:-true}"

KC_ADMIN=(kubectl -n "${KEYCLOAK_NAMESPACE}" exec deploy/"${KEYCLOAK_RELEASE}" -- /opt/keycloak/bin/kcadm.sh)

"${KC_ADMIN[@]}" config credentials \
  --server http://localhost:8080 \
  --realm master \
  --user "${ADMIN_USER}" \
  --password "${ADMIN_PASSWORD}"

if ! "${KC_ADMIN[@]}" get realms/"${KEYCLOAK_REALM}" >/dev/null 2>&1; then
  "${KC_ADMIN[@]}" create realms \
    -s realm="${KEYCLOAK_REALM}" \
    -s enabled=true \
    -s displayName="C2"
fi

roles=(
  c2_super_admin
  c2_devsecops
  c2_developer
  c2_client_readonly
  c2_field_operator
)

for role in "${roles[@]}"; do
  if ! "${KC_ADMIN[@]}" get roles/"${role}" -r "${KEYCLOAK_REALM}" >/dev/null 2>&1; then
    "${KC_ADMIN[@]}" create roles -r "${KEYCLOAK_REALM}" -s name="${role}"
  fi
done

"${KC_ADMIN[@]}" update roles/c2_super_admin -r "${KEYCLOAK_REALM}" -s composite=true >/dev/null

composite_roles=(
  c2_devsecops
  c2_developer
  c2_client_readonly
  c2_field_operator
)

for role in "${composite_roles[@]}"; do
  "${KC_ADMIN[@]}" add-roles -r "${KEYCLOAK_REALM}" --rname c2_super_admin --rolename "${role}" >/dev/null || true
done

groups=(
  c2-super-admins
  c2-devsecops
  c2-developers
  c2-clients-readonly
  c2-field-operators
)

for group in "${groups[@]}"; do
  group_lookup=$("${KC_ADMIN[@]}" get groups -r "${KEYCLOAK_REALM}" -q search="${group}")
  if ! echo "${group_lookup}" | grep -q "\"name\" : \"${group}\""; then
    "${KC_ADMIN[@]}" create groups -r "${KEYCLOAK_REALM}" -s name="${group}" >/dev/null
  fi
done

"${KC_ADMIN[@]}" add-roles -r "${KEYCLOAK_REALM}" --gname c2-super-admins --rolename c2_super_admin >/dev/null || true
"${KC_ADMIN[@]}" add-roles -r "${KEYCLOAK_REALM}" --gname c2-devsecops --rolename c2_devsecops >/dev/null || true
"${KC_ADMIN[@]}" add-roles -r "${KEYCLOAK_REALM}" --gname c2-developers --rolename c2_developer >/dev/null || true
"${KC_ADMIN[@]}" add-roles -r "${KEYCLOAK_REALM}" --gname c2-clients-readonly --rolename c2_client_readonly >/dev/null || true
"${KC_ADMIN[@]}" add-roles -r "${KEYCLOAK_REALM}" --gname c2-field-operators --rolename c2_field_operator >/dev/null || true

"${KC_ADMIN[@]}" add-roles -r "${KEYCLOAK_REALM}" --gname c2-super-admins --cclientid realm-management --rolename realm-admin >/dev/null || true
"${KC_ADMIN[@]}" add-roles -r "${KEYCLOAK_REALM}" --gname c2-devsecops --cclientid realm-management --rolename view-realm >/dev/null || true
"${KC_ADMIN[@]}" add-roles -r "${KEYCLOAK_REALM}" --gname c2-devsecops --cclientid realm-management --rolename view-users >/dev/null || true
"${KC_ADMIN[@]}" add-roles -r "${KEYCLOAK_REALM}" --gname c2-devsecops --cclientid realm-management --rolename manage-users >/dev/null || true

user_id=$("${KC_ADMIN[@]}" get users -r "${KEYCLOAK_REALM}" -q username="${SUPERADMIN_USERNAME}" | sed -n 's/.*\"id\" : \"\\([^\"]*\\)\".*/\\1/p' | head -n1)
if [[ -z "${user_id}" ]]; then
  email_id=$("${KC_ADMIN[@]}" get users -r "${KEYCLOAK_REALM}" -q email="${SUPERADMIN_EMAIL}" | sed -n 's/.*\"id\" : \"\\([^\"]*\\)\".*/\\1/p' | head -n1)
  if [[ -n "${email_id}" ]]; then
    edit_username_allowed=$("${KC_ADMIN[@]}" get realms/"${KEYCLOAK_REALM}" | sed -n 's/.*\"editUsernameAllowed\" : \\([a-z]*\\).*/\\1/p' | head -n1)
    restore_edit_username=false
    if [[ "${edit_username_allowed}" != "true" ]]; then
      "${KC_ADMIN[@]}" update realms/"${KEYCLOAK_REALM}" -s editUsernameAllowed=true
      restore_edit_username=true
    fi

    "${KC_ADMIN[@]}" update users/"${email_id}" -r "${KEYCLOAK_REALM}" \
      -s username="${SUPERADMIN_USERNAME}" \
      -s email="${SUPERADMIN_EMAIL}" \
      -s firstName="${SUPERADMIN_FIRSTNAME}" \
      -s lastName="${SUPERADMIN_LASTNAME}" \
      -s enabled=true \
      -s emailVerified=true

    if [[ "${restore_edit_username}" == "true" ]]; then
      "${KC_ADMIN[@]}" update realms/"${KEYCLOAK_REALM}" -s editUsernameAllowed=false
    fi
    user_id="${email_id}"
  else
    "${KC_ADMIN[@]}" create users -r "${KEYCLOAK_REALM}" \
      -s username="${SUPERADMIN_USERNAME}" \
      -s email="${SUPERADMIN_EMAIL}" \
      -s firstName="${SUPERADMIN_FIRSTNAME}" \
      -s lastName="${SUPERADMIN_LASTNAME}" \
      -s enabled=true \
      -s emailVerified=true
    user_id=$("${KC_ADMIN[@]}" get users -r "${KEYCLOAK_REALM}" -q username="${SUPERADMIN_USERNAME}" | sed -n 's/.*\"id\" : \"\\([^\"]*\\)\".*/\\1/p' | head -n1)
  fi
fi

temp_flag=()
if [[ "${SUPERADMIN_TEMP_PASSWORD}" == "true" ]]; then
  temp_flag=(--temporary)
fi

"${KC_ADMIN[@]}" set-password -r "${KEYCLOAK_REALM}" \
  --username "${SUPERADMIN_USERNAME}" \
  "${temp_flag[@]}" \
  --new-password "${SUPERADMIN_PASSWORD}"

for group in c2-super-admins c2-devsecops c2-developers; do
  group_id=$("${KC_ADMIN[@]}" get groups -r "${KEYCLOAK_REALM}" -q search="${group}" | sed -n 's/.*\"id\" : \"\\([^\"]*\\)\".*/\\1/p' | head -n1)
  if [[ -n "${group_id}" && -n "${user_id}" ]]; then
    "${KC_ADMIN[@]}" update users/"${user_id}"/groups/"${group_id}" -r "${KEYCLOAK_REALM}" >/dev/null || true
  fi
done

echo "Keycloak realm '${KEYCLOAK_REALM}' bootstrapped."
