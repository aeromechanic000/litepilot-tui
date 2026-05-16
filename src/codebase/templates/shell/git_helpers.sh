#!/bin/bash
# @LITE_DESC Git helper functions for branch management, rebase shortcuts, and workflow automation
# @LITE_SCENE Daily Git workflow optimization and developer productivity
# @LITE_TAGS shell, bash, git, helper, workflow

# ============================================================================
# BRANCH MANAGEMENT
# ============================================================================

# Create new branch from main and push tracking branch
git_new_branch() {
    local branch_name="$1"
    local base_branch="${2:-main}"

    if [ -z "${branch_name}" ]; then
        echo "Usage: git_new_branch <branch_name> [base_branch]"
        return 1
    fi

    git checkout "${base_branch}" && \
    git pull && \
    git checkout -b "${branch_name}" && \
    git push -u origin "${branch_name}"
}

# Delete branch locally and remotely
git_delete_branch() {
    local branch_name="$1"

    if [ -z "${branch_name}" ]; then
        echo "Usage: git_delete_branch <branch_name>"
        return 1
    fi

    git branch -d "${branch_name}"
    git push origin --delete "${branch_name}"
}

# List branches sorted by last commit date
git_list_branches() {
    git for-each-ref --sort=-committerdate refs/heads/ \
        --format='%(HEAD) %(color:yellow)%(refname:short)|%(color:bold green)%(committerdate:relative)|%(color:blue)%(authorname)%09%(color:reset)%(subject)' \
    | column -t -s '|'
}

# Rename current branch
git_rename_branch() {
    local new_name="$1"

    if [ -z "${new_name}" ]; then
        echo "Usage: git_rename_branch <new_name>"
        return 1
    fi

    local old_name=$(git branch --show-current)
    git branch -m "${new_name}"
    git push origin --delete "${old_name}"
    git push -u origin "${new_name}"
}

# ============================================================================
# INTERACTIVE REBASE SHORTCUTS
# ============================================================================

# Interactive rebase with N commits
git_rebase_n() {
    local n="${1:-5}"

    git rebase -i HEAD~"${n}"
}

# Rebase current branch onto main
git_rebase_main() {
    local current_branch=$(git branch --show-current)
    local base_branch="${1:-main}"

    git fetch origin "${base_branch}" && \
    git rebase "origin/${base_branch}" && \
    git push --force-with-lease origin "${current_branch}"
}

# Continue rebase after fixing conflicts
git_rebase_continue() {
    git add -A
    git rebase --continue
}

# Abort current rebase
git_rebase_abort() {
    git rebase --abort
}

# Auto-squash last N commits
git_squash_n() {
    local n="${1:-3}"
    local commit_msg="${2:-$(git log -1 --format=%s)}"

    git reset --soft HEAD~"${n}"
    git commit -m "${commit_msg}"
}

# ============================================================================
# STASH HELPERS
# ============================================================================

# Stash with custom message
git_stash_save() {
    local msg="$1"
    git stash push -m "${msg}"
}

# List all stashes with details
git_stash_list() {
    git stash list \
        --format='stash@{%H} %gd %s %cr' \
    | sed 's/stash@{//; s/}/:/'
}

# Show diff of specific stash
git_stash_show() {
    local stash_ref="${1:-stash@{0}}"
    git stash show -p "${stash_ref}"
}

# Apply and drop specific stash
git_stash_pop() {
    local stash_ref="${1:-stash@{0}}"
    git stash pop "${stash_ref}"
}

# Drop specific stash
git_stash_drop() {
    local stash_ref="${1:-stash@{0}}"
    git stash drop "${stash_ref}"
}

# Stash only untracked files
git_stash_untracked() {
    git stash push --include-untracked
}

# ============================================================================
# LOG FORMATS
# ============================================================================

# Pretty log with graph and decorations
git_log_pretty() {
    local n="${1:-20}"
    git log \
        --graph \
        --pretty=format:'%Cred%h%Creset -%C(yellow)%d%Creset %s %Cgreen(%cr) %C(bold blue)<%an>%Creset' \
        --abbrev-commit \
        -n "${n}"
}

# Log files changed in each commit
git_log_files() {
    local n="${1:-10}"
    git log \
        --stat \
        --pretty=format:'%C(yellow)%h%Creset %s %Cgreen(%cr)%Creset' \
        -n "${n}"
}

# Search commit messages
git_log_search() {
    local search_term="$1"
    git log --all --grep="${search_term}" --oneline
}

# Log for specific file
git_log_file() {
    local file="$1"
    git log --follow --patch -- "${file}"
}

# ============================================================================
# RELEASE TAGGING
# ============================================================================

# Create and push new tag
git_tag_release() {
    local tag_name="$1"
    local msg="${2:-Release ${tag_name}}"

    if [ -z "${tag_name}" ]; then
        echo "Usage: git_tag_release <tag_name> [message]"
        return 1
    fi

    git tag -a "${tag_name}" -m "${msg}"
    git push origin "${tag_name}"
}

# List all tags with dates
git_list_tags() {
    git tag \
        --format='%(refname:short) %09 %(authorname) %09 %(creatordate:short)' \
    | sort -V
}

# Show changes since last tag
git_show_changelog() {
    local last_tag=$(git describe --tags --abbrev=0)
    echo "Changes since ${last_tag}:"
    git log "${last_tag}..HEAD" --oneline --no-merges
}

# Changelog between two tags
git_compare_tags() {
    local tag1="$1"
    local tag2="$2"

    git log "${tag1}..${tag2}" --oneline --no-merges
}

# Delete tag locally and remotely
git_delete_tag() {
    local tag_name="$1"

    if [ -z "${tag_name}" ]; then
        echo "Usage: git_delete_tag <tag_name>"
        return 1
    fi

    git tag -d "${tag_name}"
    git push origin --delete "${tag_name}"
}

# ============================================================================
# WORKFLOW HELPERS
# ============================================================================

# Sync fork with upstream
git_sync_fork() {
    local remote="${1:-upstream}"
    local branch="${2:-main}"

    git fetch "${remote}"
    git checkout "${branch}"
    git merge "${remote}/${branch}"
    git push origin "${branch}"
}

# Clean merged branches
git_clean_merged() {
    local main_branch="${1:-main}"

    git branch --merged "${main_branch}" \
        | grep -v "^\*" \
        | grep -v "${main_branch}" \
        | xargs -r git branch -d
}

# Show contributors sorted by commit count
git_contributors() {
    git shortlog -sn --all --no-merges
}

# Find large files in repository
git_find_large() {
    local size="${1:-100M}"
    git rev-list --objects --all \
        | git cat-file --batch-check='%(objecttype) %(objectname) %(objectsize) %(rest)' \
        | awk '/^blob/ {print substr($0,6)}' \
        | sort -n -k2 \
        | numfmt --field=2 --to=iec \
        | tail -20
}

# Undo last commit but keep changes
git_undo_commit() {
    git reset --soft HEAD~1
}

# Amend last commit with new message
git_amend_msg() {
    local new_msg="$1"
    git commit --amend -m "${new_msg}"
}

# Show files with conflicts
git_conflicts() {
    git diff --name-only --diff-filter=U
}

# Export git config
git_export_config() {
    git config --global --list
}

# Import source completion
if [ -n "${BASH_VERSION}" ]; then
    # Source git completion if available
    if [ -f /usr/share/bash-completion/completions/git ]; then
        source /usr/share/bash-completion/completions/git
    fi
fi