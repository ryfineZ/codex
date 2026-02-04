#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [[ "${CODEX_SKIP_UPDATE:-}" != "1" ]]; then
  if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    stash_name=""
    if [[ -n "$(git status --porcelain)" ]]; then
      stash_name="codex-autoupdate-$(date +%Y%m%d-%H%M%S)"
      echo "Working tree dirty; stashing changes ($stash_name)..." >&2
      git stash push -u -m "$stash_name" >/dev/null
    fi

    if upstream=$(git rev-parse --abbrev-ref --symbolic-full-name '@{u}' 2>/dev/null); then
      update_mode="${CODEX_AUTO_UPDATE_MODE:-safe}"
      if [[ "$update_mode" != "never" ]]; then
        if git fetch --prune; then
          base="$(git merge-base HEAD "$upstream" || true)"
          incoming_files=""
          if [[ -n "$base" ]]; then
            incoming_files="$(git diff --name-only "$base".."$upstream")"
          else
            echo "No merge base found; updating without safety check." >&2
            incoming_files="__unknown__"
          fi

          if [[ -n "$incoming_files" && "$incoming_files" != "__unknown__" ]]; then
            echo "Incoming updates from $upstream:" >&2
            git log --oneline "$base".."$upstream" | head -n 20 >&2 || true
          else
            if [[ "$incoming_files" != "__unknown__" ]]; then
              echo "Already up to date with $upstream." >&2
            fi
          fi

          skip_update=""
          if [[ "$update_mode" == "safe" && -n "$base" && -n "$incoming_files" && "$incoming_files" != "__unknown__" ]]; then
            local_files="$(git diff --name-only "$base"..HEAD)"
            conflict_files="$(comm -12 <(printf '%s\n' "$incoming_files" | sort) <(printf '%s\n' "$local_files" | sort) || true)"
            if [[ -n "$conflict_files" ]]; then
              echo "Incoming changes touch local files; skipping auto-update." >&2
              echo "$conflict_files" | sed 's/^/  - /' >&2
              skip_update="1"
            fi
          fi

          if [[ -z "$skip_update" && -n "$incoming_files" ]]; then
            echo "Updating from $upstream..." >&2
            if ! git pull --rebase; then
              echo "Auto-update failed; continuing with current version." >&2
            fi
          fi
        else
          echo "Fetch failed; continuing with current version." >&2
        fi
      else
        echo "Auto-update disabled by CODEX_AUTO_UPDATE_MODE=never." >&2
      fi
    else
      echo "No upstream configured; skipping auto-update." >&2
    fi

    if [[ -n "$stash_name" ]]; then
      if git stash list | grep -q "$stash_name"; then
        echo "Restoring stashed changes ($stash_name)..." >&2
        if ! git stash pop >/dev/null; then
          echo "Stash pop had conflicts; resolve manually if needed." >&2
        fi
      fi
    fi
  else
    echo "Not a git repo; skipping auto-update." >&2
  fi
fi

cd "$ROOT/codex-rs"
exec cargo run -p codex-tui --bin codex-tui -- "$@"
