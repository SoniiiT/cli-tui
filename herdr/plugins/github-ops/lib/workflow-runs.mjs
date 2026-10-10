import { jsonCommand, run } from './process.mjs'

export function buildListWorkflowsArgs(repo) {
  return ['workflow', 'list', '--repo', repo, '--json', 'id,name,state,path', '--limit', '50']
}

export function listWorkflows(repo, cwd) {
  return jsonCommand('gh', buildListWorkflowsArgs(repo), { cwd })
}

export function buildListRunsArgs(repo, { workflow, branch, status } = {}) {
  const args = [
    'run', 'list', '--repo', repo, '--limit', '30',
    '--json', 'databaseId,name,displayTitle,status,conclusion,headBranch,headSha,event,workflowName,createdAt,updatedAt',
  ]
  if (workflow) args.push('--workflow', String(workflow))
  if (branch) args.push('--branch', branch)
  if (status) args.push('--status', status)
  return args
}

export function listRuns(repo, cwd, filters) {
  return jsonCommand('gh', buildListRunsArgs(repo, filters), { cwd })
}

export function buildViewRunArgs(target) {
  return [
    'run', 'view', String(target.number), '--repo', target.nameWithOwner,
    '--json', 'status,conclusion,jobs,headBranch,headSha,event,url,displayTitle,workflowName,createdAt,updatedAt',
  ]
}

export function viewRun(target, cwd) {
  return jsonCommand('gh', buildViewRunArgs(target), { cwd })
}

export function buildRunLogsArgs(target, { failedOnly = true } = {}) {
  return [
    'run', 'view', String(target.number), '--repo', target.nameWithOwner,
    failedOnly ? '--log-failed' : '--log',
  ]
}

export function runLogs(target, cwd, opts) {
  return run('gh', buildRunLogsArgs(target, opts), { cwd, allowFailure: true }).stdout
}

export function buildRerunArgs(target, { failedOnly = false } = {}) {
  const args = ['run', 'rerun', String(target.number), '--repo', target.nameWithOwner]
  if (failedOnly) args.push('--failed')
  return args
}

export function rerunRun(target, cwd, opts) {
  return run('gh', buildRerunArgs(target, opts), { cwd })
}

export function buildCancelArgs(target, { force = false } = {}) {
  const args = ['run', 'cancel', String(target.number), '--repo', target.nameWithOwner]
  if (force) args.push('--force')
  return args
}

export function cancelRun(target, cwd, opts) {
  return run('gh', buildCancelArgs(target, opts), { cwd })
}

export function buildTriggerWorkflowArgs(repo, workflow, { ref, fields = [] } = {}) {
  const args = ['workflow', 'run', String(workflow), '--repo', repo]
  if (ref) args.push('--ref', ref)
  for (const [key, value] of fields) args.push('-f', `${key}=${value}`)
  return args
}

export function triggerWorkflow(repo, workflow, cwd, opts) {
  return run('gh', buildTriggerWorkflowArgs(repo, workflow, opts), { cwd })
}
