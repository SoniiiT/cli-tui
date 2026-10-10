export function parseGithubTarget(input) {
  const value = String(input ?? '').trim()
  const match = /^https:\/\/github\.com\/([^/]+)\/([^/]+)\/(issues|pull|actions\/runs)\/(\d+)(?:[/?#].*)?$/i.exec(value)
  if (!match) return null
  const kindPath = match[3].toLowerCase()
  const kind = kindPath === 'pull' ? 'pull' : kindPath === 'issues' ? 'issue' : 'run'
  const repo = match[2].replace(/\.git$/i, '')
  const path = kind === 'pull' ? 'pull' : kind === 'issue' ? 'issues' : 'actions/runs'
  return {
    owner: match[1],
    repo,
    nameWithOwner: `${match[1]}/${repo}`,
    kind,
    number: Number(match[4]),
    url: `https://github.com/${match[1]}/${repo}/${path}/${match[4]}`,
  }
}

export function slug(value, limit = 40) {
  return String(value ?? '')
    .normalize('NFKD')
    .replace(/[\u0300-\u036f]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, limit)
    .replace(/-+$/g, '') || 'item'
}

export function worktreeBranch(target, title, nonce = Date.now().toString(36), mode = 'work') {
  const kind = target.kind === 'pull' && mode === 'fix' ? 'pr-fix' : target.kind
  return `github-ops-${kind}-${target.number}-${slug(title, 28)}-${slug(nonce, 8)}`
}

export function delegatedPrompt(target, title, mode = 'work') {
  if (target.kind === 'pull') {
    if (mode === 'fix') {
      return `Prepare follow-up changes for GitHub pull request #${target.number}: ${title}. This worktree starts from the PR head (${target.url}). Make local changes only. Do not commit, push, approve, comment, or merge; leave the result for the user to review.`
    }
    return `Review GitHub pull request #${target.number}: ${title}. Inspect it with gh pr view and gh pr diff (${target.url}). Report actionable findings with file and line references. Do not approve, comment, merge, push, or submit changes.`
  }
  if (target.kind === 'run') {
    return `Investigate the failed GitHub Actions run #${target.number} (${target.workflowName ?? 'workflow'}) on ${target.headBranch ?? '?'}@${target.headSha ?? '?'} for ${target.nameWithOwner}. This worktree starts from the run's commit. The failing step's log output is included below; diagnose the cause and fix it in this isolated worktree. Do not rerun, cancel, push, approve, or merge; leave the result for the user to review.\n\n${target.failedLog ?? '(no log captured)'}`
  }
  if (target.kind === 'release') {
    return `Draft release notes for ${target.nameWithOwner}, release ${target.number}${target.previousTag ? ` (covering commits and merged PRs since ${target.previousTag})` : ''}. Use gh pr list --search and git log to gather context. Output only the drafted markdown notes in your response. Do not create, edit, publish, or delete any GitHub release, tag, or git ref yourself.`
  }
  return `Work on GitHub issue #${target.number}: ${title}. Read the issue with gh issue view (${target.url}), implement the requested change in this isolated worktree, and leave your changes for review. Do not commit, push, comment, close, approve, or merge.`
}
