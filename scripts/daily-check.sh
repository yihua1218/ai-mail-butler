#!/usr/bin/env bash
set -u

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

PUBLIC_URL="${DAILY_CHECK_PUBLIC_URL:-https://butler-dev.yihua.app}"
CHECK_EMAIL="${DAILY_CHECK_EMAIL:-yihua1218@gmail.com}"
CONTAINER_NAME="${DAILY_CHECK_CONTAINER_NAME:-ai-mail-butler}"
EXPECTED_MODEL="${DAILY_CHECK_EXPECTED_MODEL:-google/gemma-4-31b-qat}"
DB_PATH="${DAILY_CHECK_DB_PATH:-${REPO_ROOT}/ai-mail-butler-data/overlay/data/data.sqlite}"
REPORT_DIR="${DAILY_CHECK_REPORT_DIR:-${REPO_ROOT}/logs/daily-checks}"
MIN_UBER_COUNT="${DAILY_CHECK_MIN_UBER_COUNT:-66}"
MIN_UBER_SUM="${DAILY_CHECK_MIN_UBER_SUM:-25328}"
MAX_SUMMARY_DELTA="${DAILY_CHECK_MAX_SUMMARY_DELTA:-0.01}"
SQL_EMAIL="$(printf "%s" "${CHECK_EMAIL}" | sed "s/'/''/g")"

mkdir -p "${REPORT_DIR}"

RUN_ID="$(date -u +"%Y%m%dT%H%M%SZ")"
REPORT_PATH="${DAILY_CHECK_REPORT_PATH:-${REPORT_DIR}/${RUN_ID}.json}"
TMP_DIR="$(mktemp -d)"
FAILURES=()

cleanup() {
    rm -rf "${TMP_DIR}"
}
trap cleanup EXIT

add_failure() {
    FAILURES+=("$1")
}

http_get() {
    local url="$1"
    local output="$2"
    curl -sS -m 60 -o "${output}" -w "%{http_code}" "${url}" 2>"${output}.err" || true
}

email_urlencoded="$(jq -rn --arg value "${CHECK_EMAIL}" '$value|@uri')"

root_body="${TMP_DIR}/root.html"
finance_body="${TMP_DIR}/finance-records.json"
monthly_body="${TMP_DIR}/finance-monthly.json"

root_status="$(http_get "${PUBLIC_URL%/}/" "${root_body}")"
finance_status="$(http_get "${PUBLIC_URL%/}/api/finance/records?email=${email_urlencoded}" "${finance_body}")"
monthly_status="$(http_get "${PUBLIC_URL%/}/api/finance/monthly?email=${email_urlencoded}" "${monthly_body}")"

if [[ "${root_status}" != "200" ]]; then
    add_failure "root HTTP status was ${root_status}"
fi
if [[ "${finance_status}" != "200" ]]; then
    add_failure "finance records HTTP status was ${finance_status}"
fi
if [[ "${monthly_status}" != "200" ]]; then
    add_failure "finance monthly HTTP status was ${monthly_status}"
fi

container_status="unavailable"
container_image=""
container_id=""
if command -v nerdctl >/dev/null 2>&1; then
    container_json="${TMP_DIR}/container.jsonl"
    if nerdctl ps --format json --filter "name=${CONTAINER_NAME}" >"${container_json}" 2>"${container_json}.err"; then
        container_status="$(jq -r --arg name "${CONTAINER_NAME}" 'select(.Names == $name or (.Names | contains($name))) | .Status' "${container_json}" | head -n 1)"
        container_image="$(jq -r --arg name "${CONTAINER_NAME}" 'select(.Names == $name or (.Names | contains($name))) | .Image' "${container_json}" | head -n 1)"
        container_id="$(jq -r --arg name "${CONTAINER_NAME}" 'select(.Names == $name or (.Names | contains($name))) | .ID' "${container_json}" | head -n 1)"
    fi
else
    add_failure "nerdctl was not found"
fi
if [[ -z "${container_status}" || "${container_status}" == "null" || "${container_status}" == "unavailable" ]]; then
    add_failure "container ${CONTAINER_NAME} was not running"
fi

actual_model=""
if command -v nerdctl >/dev/null 2>&1; then
    actual_model="$(nerdctl exec "${CONTAINER_NAME}" printenv AI_MODEL_NAME 2>/dev/null || true)"
fi
if [[ "${actual_model}" != "${EXPECTED_MODEL}" ]]; then
    add_failure "AI model was '${actual_model:-missing}', expected '${EXPECTED_MODEL}'"
fi

finance_count=0
api_uber_count=0
api_uber_sum=0
api_uber_by_type="[]"
if [[ "${finance_status}" == "200" ]] && jq -e '.records | type == "array"' "${finance_body}" >/dev/null 2>&1; then
    finance_count="$(jq '.records | length' "${finance_body}")"
    api_uber_count="$(jq '[.records[] | select((.finance_type // "") | startswith("uber"))] | length' "${finance_body}")"
    api_uber_sum="$(jq '[.records[] | select((.finance_type // "") | startswith("uber")) | .amount] | add // 0' "${finance_body}")"
    api_uber_by_type="$(jq -c '[.records[] | select((.finance_type // "") | startswith("uber"))] | group_by(.finance_type) | map({type:.[0].finance_type,count:length,sum:(map(.amount)|add)})' "${finance_body}")"
    if ! jq -n --argjson actual "${api_uber_count}" --argjson expected "${MIN_UBER_COUNT}" '$actual >= $expected' >/dev/null; then
        add_failure "Uber finance record count ${api_uber_count} was below ${MIN_UBER_COUNT}"
    fi
    if ! jq -n --argjson actual "${api_uber_sum}" --argjson expected "${MIN_UBER_SUM}" '$actual >= $expected' >/dev/null; then
        add_failure "Uber finance sum ${api_uber_sum} was below ${MIN_UBER_SUM}"
    fi
else
    add_failure "finance records response did not contain records array"
fi

monthly_expense="[]"
if [[ "${monthly_status}" == "200" ]] && jq -e '.monthly | type == "array"' "${monthly_body}" >/dev/null 2>&1; then
    monthly_expense="$(jq -c '[.monthly[] | select(.category == "expense") | {month_key,total_amount}]' "${monthly_body}")"
else
    add_failure "finance monthly response did not contain monthly array"
fi

db_exists=false
db_uber_count=0
db_uber_sum=0
summary_diff_count=0
summary_max_abs_delta=0
if [[ -f "${DB_PATH}" ]] && command -v sqlite3 >/dev/null 2>&1; then
    db_exists=true
    amount_expr="amount"
    if sqlite3 "${DB_PATH}" "PRAGMA table_info(email_financial_records);" | awk -F'|' '$2 == "amount_twd" { found = 1 } END { exit !found }'; then
        amount_expr="COALESCE(NULLIF(amount_twd, 0), amount)"
    fi
    db_uber_count="$(sqlite3 "${DB_PATH}" "SELECT COUNT(*) FROM email_financial_records WHERE user_id=(SELECT id FROM users WHERE email='${SQL_EMAIL}') AND finance_type LIKE 'uber%';")"
    db_uber_sum="$(sqlite3 "${DB_PATH}" "SELECT COALESCE(SUM(${amount_expr}), 0) FROM email_financial_records WHERE user_id=(SELECT id FROM users WHERE email='${SQL_EMAIL}') AND finance_type LIKE 'uber%';")"
    summary_query="
WITH record_totals AS (
  SELECT user_id, month_key, category, ROUND(SUM(${amount_expr}), 2) AS records_total
  FROM email_financial_records
  WHERE user_id=(SELECT id FROM users WHERE email='${SQL_EMAIL}')
  GROUP BY user_id, month_key, category
)
SELECT
  COUNT(*),
  COALESCE(MAX(ABS(ROUND(s.total_amount, 2) - COALESCE(r.records_total, 0))), 0)
FROM monthly_finance_summary s
LEFT JOIN record_totals r
  ON r.user_id = s.user_id
 AND r.month_key = s.month_key
 AND r.category = s.category
WHERE s.user_id=(SELECT id FROM users WHERE email='${SQL_EMAIL}')
  AND ABS(ROUND(s.total_amount, 2) - COALESCE(r.records_total, 0)) > ${MAX_SUMMARY_DELTA};
"
    summary_result="$(sqlite3 "${DB_PATH}" "${summary_query}")"
    summary_diff_count="${summary_result%%|*}"
    summary_max_abs_delta="${summary_result##*|}"

    if ! jq -n --argjson actual "${db_uber_count}" --argjson expected "${MIN_UBER_COUNT}" '$actual >= $expected' >/dev/null; then
        add_failure "DB Uber finance record count ${db_uber_count} was below ${MIN_UBER_COUNT}"
    fi
    if ! jq -n --argjson actual "${db_uber_sum}" --argjson expected "${MIN_UBER_SUM}" '$actual >= $expected' >/dev/null; then
        add_failure "DB Uber finance sum ${db_uber_sum} was below ${MIN_UBER_SUM}"
    fi
    if [[ "${summary_diff_count}" != "0" ]]; then
        add_failure "monthly summary had ${summary_diff_count} mismatched rows, max delta ${summary_max_abs_delta}"
    fi
else
    if [[ ! -f "${DB_PATH}" ]]; then
        add_failure "DB path was not found: ${DB_PATH}"
    else
        add_failure "sqlite3 was not found"
    fi
fi

status="success"
if ((${#FAILURES[@]} > 0)); then
    status="failure"
fi

failures_json="$(printf '%s\n' "${FAILURES[@]}" | jq -Rsc 'split("\n") | map(select(length > 0))')"

jq -n \
    --arg status "${status}" \
    --arg run_id "${RUN_ID}" \
    --arg checked_at "$(date -u +"%Y-%m-%dT%H:%M:%SZ")" \
    --arg public_url "${PUBLIC_URL}" \
    --arg email "${CHECK_EMAIL}" \
    --arg container_name "${CONTAINER_NAME}" \
    --arg container_status "${container_status}" \
    --arg container_image "${container_image}" \
    --arg container_id "${container_id}" \
    --arg expected_model "${EXPECTED_MODEL}" \
    --arg actual_model "${actual_model}" \
    --arg root_status "${root_status}" \
    --arg finance_status "${finance_status}" \
    --arg monthly_status "${monthly_status}" \
    --arg db_path "${DB_PATH}" \
    --argjson db_exists "${db_exists}" \
    --argjson finance_count "${finance_count}" \
    --argjson api_uber_count "${api_uber_count}" \
    --argjson api_uber_sum "${api_uber_sum}" \
    --argjson api_uber_by_type "${api_uber_by_type}" \
    --argjson monthly_expense "${monthly_expense}" \
    --argjson db_uber_count "${db_uber_count}" \
    --argjson db_uber_sum "${db_uber_sum}" \
    --argjson summary_diff_count "${summary_diff_count}" \
    --argjson summary_max_abs_delta "${summary_max_abs_delta}" \
    --argjson min_uber_count "${MIN_UBER_COUNT}" \
    --argjson min_uber_sum "${MIN_UBER_SUM}" \
    --argjson max_summary_delta "${MAX_SUMMARY_DELTA}" \
    --argjson failures "${failures_json}" \
    '{
      status: $status,
      run_id: $run_id,
      checked_at: $checked_at,
      target: {
        public_url: $public_url,
        email: $email
      },
      checks: {
        http: {
          root_status: ($root_status | tonumber? // $root_status),
          finance_records_status: ($finance_status | tonumber? // $finance_status),
          finance_monthly_status: ($monthly_status | tonumber? // $monthly_status)
        },
        container: {
          name: $container_name,
          status: $container_status,
          image: $container_image,
          id: $container_id
        },
        model: {
          expected: $expected_model,
          actual: $actual_model
        },
        finance_api: {
          record_count: $finance_count,
          uber_count: $api_uber_count,
          uber_sum: $api_uber_sum,
          uber_by_type: $api_uber_by_type,
          monthly_expense: $monthly_expense
        },
        finance_db: {
          db_path: $db_path,
          db_exists: $db_exists,
          uber_count: $db_uber_count,
          uber_sum: $db_uber_sum,
          monthly_summary_mismatch_count: $summary_diff_count,
          monthly_summary_max_abs_delta: $summary_max_abs_delta
        }
      },
      thresholds: {
        min_uber_count: $min_uber_count,
        min_uber_sum: $min_uber_sum,
        max_summary_delta: $max_summary_delta
      },
      failures: $failures
    }' >"${REPORT_PATH}"

echo "${REPORT_PATH}"

if [[ "${status}" != "success" ]]; then
    jq '.failures' "${REPORT_PATH}" >&2
    exit 1
fi
