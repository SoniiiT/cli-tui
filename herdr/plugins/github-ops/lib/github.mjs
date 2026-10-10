import { jsonCommand, run } from './process.mjs'

export function repository(cwd, explicit = null) {
  const args = ['repo', 'view']
  if (explicit) args.push(explicit)
  args.push('--json', 'nameWithOwner,defaultBranchRef,url')
  const value = jsonCommand('gh', args, { cwd })
  return {
    name: value.nameWithOwner,
    url: value.url,
    defaultBranch: value.defaultBranchRef?.name ?? 'main',
  }
}

export function listPullRequests(repo, cwd, state = 'open') {
  return jsonCommand('gh', [
    'pr', 'list', '--repo', repo, '--state', state, '--limit', '40',
    '--json', 'number,title,state,url,author,reviewDecision,isDraft,updatedAt',
  ], { cwd })
}

export function listIssues(repo, cwd, state = 'open') {
  return jsonCommand('gh', [
    'issue', 'list', '--repo', repo, '--state', state, '--limit', '40',
    '--json', 'number,title,state,url,author,labels,assignees,updatedAt',
  ], { cwd })
}

export function viewTarget(target, cwd) {
  const fields = target.kind === 'pull'
    ? 'number,title,state,body,url,author,labels,assignees,reviewDecision,isDraft,mergeStateStatus,headRefName,baseRefName,comments,reviews,files,statusCheckRollup'
    : 'number,title,state,body,url,author,labels,assignees,comments,createdAt,updatedAt'
  return jsonCommand('gh', [
    target.kind === 'pull' ? 'pr' : 'issue', 'view', String(target.number),
    '--repo', target.nameWithOwner, '--json', fields,
  ], { cwd })
}

export function commentTarget(target, cwd, body) {
  return run('gh', [
    target.kind === 'pull' ? 'pr' : 'issue', 'comment', String(target.number),
    '--repo', target.nameWithOwner, '--body', body,
  ], { cwd })
}

export function editTarget(target, cwd, title, body) {
  const args = [target.kind === 'pull' ? 'pr' : 'issue', 'edit', String(target.number), '--repo', target.nameWithOwner]
  if (title !== null) args.push('--title', title)
  if (body !== null) args.push('--body', body)
  return run('gh', args, { cwd })
}

export function buildReviewArgs(target, event, body = '') {
  const eventFlag = event === 'approve' ? '--approve' : '--request-changes'
  const args = ['pr', 'review', String(target.number), '--repo', target.nameWithOwner, eventFlag]
  if (body.trim()) args.push('--body', body)
  return args
}

export function reviewPullRequest(target, cwd, event, body = '') {
  return run('gh', buildReviewArgs(target, event, body), { cwd })
}

export function buildMergeArgs(target, method) {
  return [
    'pr', 'merge', String(target.number), '--repo', target.nameWithOwner, `--${method}`, '--disable-auto',
  ]
}

export function mergePullRequest(target, cwd, method) {
  return run('gh', buildMergeArgs(target, method), { cwd })
}

export function closeOrReopen(target, cwd, reopen = false) {
  return run('gh', [
    target.kind === 'pull' ? 'pr' : 'issue', reopen ? 'reopen' : 'close',
    String(target.number), '--repo', target.nameWithOwner,
  ], { cwd })
}
