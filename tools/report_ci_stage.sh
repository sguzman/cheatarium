#!/usr/bin/env bash
# Report a successful CI stage on the exact Git commit being validated.
# This permits remote status inspection without inferring results from
# absent PR-only workflow listings. Failure remains visible through
# cheatarium/build-index's independent always() status step.
set -euo pipefail

stage="${1:-}"
state="${2:-success}"
case "$state" in success|failure|error|pending) ;; *)
  echo "Invalid CI status: $state" >&2; exit 2 ;;
esac
case "$stage" in
  python-tests|source-inventory|rust-tests|index-generated|distribution-audited|consumer-smoke|test-effect-reviews|test-snes-compositions|test-snes-review-queue|test-inspect-snes-joins|test-snes-publications|test-audit-snes-publication-archive|test-analyze-snes-patterns|consumer-catalog|consumer-cli|consumer-fingerprint|consumer-zip|consumer-codecs)
    ;;
  *)
    echo "Unknown Cheatarium CI stage: $stage" >&2
    exit 2
    ;;
esac

: "${GITHUB_TOKEN:?GITHUB_TOKEN is required}"
: "${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required}"
: "${GITHUB_SHA:?GITHUB_SHA is required}"

payload="$(printf '{"state":"%s","context":"cheatarium/stage/%s","description":"Cheatarium CI phase: %s"}' "$state" "$stage" "$stage")"
curl --fail-with-body --silent --show-error --retry 2 \
  --request POST \
  --header "Accept: application/vnd.github+json" \
  --header "Authorization: Bearer ${GITHUB_TOKEN}" \
  --header "X-GitHub-Api-Version: 2022-11-28" \
  --data "$payload" \
  "https://api.github.com/repos/${GITHUB_REPOSITORY}/statuses/${GITHUB_SHA}" >/dev/null
