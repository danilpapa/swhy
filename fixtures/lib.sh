# shellcheck shell=bash
#
# Fixture framework: small git repositories with deterministic SHAs.
#
# A fixture is a script fixtures/fx-<name>.sh that takes the destination
# directory as $1, sources this file and calls the fx_* functions:
#
#     . "$(dirname "$0")/lib.sh"
#     fx_init "$1"
#     fx_write Sources/A.swift <<'EOF'
#     func a() {}
#     EOF
#     fx_commit "Add a" "2024-01-10T10:00:00+00:00"
#
# Determinism: author, committer and e-mail are fixed here; every commit gets
# its date from the script through GIT_AUTHOR_DATE and GIT_COMMITTER_DATE; the
# machine's git configuration, time zone and locale are switched off. The same
# script therefore produces the same commit SHAs on every machine.
#
# Compatible with bash 3.2 (the /bin/bash of macOS).

set -euo pipefail

FX_AUTHOR_NAME="Fixture Author"
FX_AUTHOR_EMAIL="fixture@swhy.test"

# Nothing from the caller's git environment may leak into the fixture.
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_OBJECT_DIRECTORY \
    GIT_ALTERNATE_OBJECT_DIRECTORIES GIT_COMMON_DIR GIT_NAMESPACE \
    GIT_DEFAULT_HASH GIT_TEMPLATE_DIR GIT_CONFIG GIT_CONFIG_COUNT \
    GIT_CONFIG_PARAMETERS GIT_AUTHOR_DATE GIT_COMMITTER_DATE GIT_EDITOR \
    GIT_SEQUENCE_EDITOR GIT_MERGE_AUTOEDIT

export GIT_CONFIG_GLOBAL=/dev/null
export GIT_CONFIG_SYSTEM=/dev/null
export GIT_CONFIG_NOSYSTEM=1
export GIT_TERMINAL_PROMPT=0
export GIT_AUTHOR_NAME="$FX_AUTHOR_NAME"
export GIT_AUTHOR_EMAIL="$FX_AUTHOR_EMAIL"
export GIT_COMMITTER_NAME="$FX_AUTHOR_NAME"
export GIT_COMMITTER_EMAIL="$FX_AUTHOR_EMAIL"
export TZ=UTC
export LC_ALL=C
export LANG=C

_fx_die() {
    echo "fixtures/lib.sh: $*" >&2
    exit 1
}

# Dates must carry an explicit UTC offset: 2024-01-10T10:00:00+00:00.
_fx_check_date() {
    local re='^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}[+-][0-9]{2}:[0-9]{2}$'
    [[ $1 =~ $re ]] || _fx_die "date must look like 2024-01-10T10:00:00+00:00, got '$1'"
}

# Paths are relative to the repository root and never leave it.
_fx_check_path() {
    case $1 in
        "" | /* | .. | ../* | */.. | */../*) _fx_die "path must be relative and inside the repository, got '$1'" ;;
    esac
}

_fx_in_repo() {
    [ -n "${FX_REPO:-}" ] || _fx_die "$1: call fx_init first"
}

# fx_init <dir>
# Creates an empty repository in <dir> (branch main) and makes it the current
# directory. <dir> must not exist or must be empty.
fx_init() {
    [ $# -eq 1 ] || _fx_die "usage: fx_init <dir>"
    local dir=$1
    if [ -e "$dir" ] && [ -n "$(ls -A "$dir")" ]; then
        _fx_die "fx_init: '$dir' exists and is not empty"
    fi
    mkdir -p "$dir"
    git init --quiet --initial-branch=main --template= "$dir"
    cd "$dir"
    FX_REPO=$(pwd)
    git config user.name "$FX_AUTHOR_NAME"
    git config user.email "$FX_AUTHOR_EMAIL"
    git config commit.gpgsign false
    git config tag.gpgsign false
    git config core.autocrlf false
    git config gc.auto 0
}

# fx_write <path> <<EOF
# Writes stdin to <path>, creating parent directories.
fx_write() {
    [ $# -eq 1 ] || _fx_die "usage: fx_write <path> <<EOF"
    _fx_in_repo fx_write
    _fx_check_path "$1"
    mkdir -p "$(dirname "$1")"
    cat >"$1"
}

# fx_commit <msg> <date>
# Stages every change in the working tree and commits it. Fails when there is
# nothing to commit.
fx_commit() {
    [ $# -eq 2 ] || _fx_die "usage: fx_commit <msg> <date>"
    _fx_in_repo fx_commit
    _fx_check_date "$2"
    git add -A
    GIT_AUTHOR_DATE="$2" GIT_COMMITTER_DATE="$2" git commit --quiet -m "$1"
}

# fx_mv <from> <to>
fx_mv() {
    [ $# -eq 2 ] || _fx_die "usage: fx_mv <from> <to>"
    _fx_in_repo fx_mv
    _fx_check_path "$1"
    _fx_check_path "$2"
    mkdir -p "$(dirname "$2")"
    git mv "$1" "$2"
}

# fx_rm <path>
fx_rm() {
    [ $# -eq 1 ] || _fx_die "usage: fx_rm <path>"
    _fx_in_repo fx_rm
    _fx_check_path "$1"
    git rm --quiet -r "$1"
}

# fx_branch <name>
# Creates a branch at the current commit and switches to it.
fx_branch() {
    [ $# -eq 1 ] || _fx_die "usage: fx_branch <name>"
    _fx_in_repo fx_branch
    git checkout --quiet -b "$1"
}

# fx_checkout <name>
fx_checkout() {
    [ $# -eq 1 ] || _fx_die "usage: fx_checkout <name>"
    _fx_in_repo fx_checkout
    git checkout --quiet "$1"
}

# fx_merge <branch> <msg> <date>
# Merges <branch> into the current branch with a merge commit (no
# fast-forward). A conflict is an error in the fixture script.
fx_merge() {
    [ $# -eq 3 ] || _fx_die "usage: fx_merge <branch> <msg> <date>"
    _fx_in_repo fx_merge
    _fx_check_date "$3"
    GIT_AUTHOR_DATE="$3" GIT_COMMITTER_DATE="$3" git merge --quiet --no-ff -m "$2" "$1"
}
