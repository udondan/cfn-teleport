#!/usr/bin/env bash
# Fails if the working tree introduces code smells that do not exist at the
# given base ref. `qlty smells --upstream` only narrows to changed files and
# still reports every pre-existing smell in them, so we compare both states.
# Smells are matched by rule, file and structural hash (duplication) or
# message without counts (structure), so shifted line numbers do not matter.
set -euo pipefail

base_ref="${1:?usage: qlty-new-smells.sh <base-ref>}"
tmp="$(mktemp -d)"
cleanup() {
  git worktree remove --force "$tmp/base" >/dev/null 2>&1 || true
  rm -rf "$tmp"
}
trap cleanup EXIT

git worktree add --quiet --detach "$tmp/base" "$base_ref"
(cd "$tmp/base" && qlty smells --all --json --quiet 2>/dev/null) >"$tmp/base.json"
qlty smells --all --json --quiet 2>/dev/null >"$tmp/head.json"

jq -r --slurpfile base "$tmp/base.json" '
  def key: .ruleKey + "|" + .location.path + "|"
    + (.properties.structural_hash
       // (.message | gsub(" ?\\([a-z]+ = [0-9.]+\\)"; "")));
  def counts: map(key) | group_by(.) | map({key: .[0], value: length}) | from_entries;
  ($base[0] | counts) as $b
  | . as $head
  | ($head | counts)
  | to_entries[]
  | select(.value > ($b[.key] // 0))
  | (.value - ($b[.key] // 0)) as $new
  | .key as $k
  | [ $head[] | select(key == $k) ] as $items
  | if ($items | length) == $new then
      $items[] | "::error file=\(.location.path),line=\(.location.range.startLine)::\(.message)"
    else
      "::error file=\($items[0].location.path)::\($new) new: \($items[0].message | gsub(" ?\\([a-z]+ = [0-9.]+\\)"; ""))"
    end
' "$tmp/head.json" >"$tmp/new.txt"

if [ -s "$tmp/new.txt" ]; then
  cat "$tmp/new.txt"
  echo "qlty: $(wc -l <"$tmp/new.txt" | tr -d ' ') new code smell(s) compared to $base_ref"
  exit 1
fi
echo "qlty: no new code smells compared to $base_ref"
