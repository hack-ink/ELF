project_enabled() {
  local project="$1"

  if [[ -z "${PROJECT_FILTER}" || "${PROJECT_FILTER}" == "all" ]]; then
    return 0
  fi

  for selected in ${PROJECT_FILTER//,/ }; do
    if [[ "${selected}" == "${project}" ]]; then
      return 0
    fi
  done

  return 1
}

run_project() {
  local project="$1"
  local fn="$2"

  if project_enabled "${project}"; then
    CURRENT_PROJECT_STARTED_AT="$(date +%s)"
    "${fn}"
    CURRENT_PROJECT_STARTED_AT=""
  fi
}
