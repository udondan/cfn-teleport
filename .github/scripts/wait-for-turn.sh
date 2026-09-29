#!/usr/bin/env bash
# Wait until no earlier run of the same workflow is still active.
#
# Runs are ordered by the start time of their current attempt (re-runs queue
# at the end, not at their original position), with the run ID as tie-breaker.
# Cancelled or crashed runs count as completed, so nothing can block forever.
#
# Requires GH_TOKEN with actions:read and the default GITHUB_* env variables.
set -euo pipefail

interval="${WAIT_INTERVAL_SECONDS:-30}"
workflow_id=$(gh api "repos/$GITHUB_REPOSITORY/actions/runs/$GITHUB_RUN_ID" --jq .workflow_id)
started_at=$(gh api "repos/$GITHUB_REPOSITORY/actions/runs/$GITHUB_RUN_ID" --jq .run_started_at)
echo "This run: $GITHUB_RUN_ID (attempt started $started_at)"

while true; do
  ahead=$(
    for status in queued in_progress waiting requested pending; do
      gh api --paginate "repos/$GITHUB_REPOSITORY/actions/workflows/$workflow_id/runs?status=$status&per_page=100" \
        --jq ".workflow_runs[]
              | select(.id != $GITHUB_RUN_ID)
              | select(.run_started_at < \"$started_at\" or (.run_started_at == \"$started_at\" and .id < $GITHUB_RUN_ID))
              | \"\(.id) \(.head_branch) \(.status)\""
    done
  )
  if [ -z "$ahead" ]; then
    echo "No earlier run active, continuing."
    exit 0
  fi
  echo "$(date -u +%H:%M:%S) waiting for:"
  echo "$ahead" | sed 's/^/  /'
  sleep "$interval"
done
