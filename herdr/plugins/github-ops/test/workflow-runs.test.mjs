import assert from 'node:assert/strict'
import test from 'node:test'
import {
  buildCancelArgs,
  buildListRunsArgs,
  buildListWorkflowsArgs,
  buildRerunArgs,
  buildRunLogsArgs,
  buildTriggerWorkflowArgs,
  buildViewRunArgs,
} from '../lib/workflow-runs.mjs'

const target = { kind: 'run', number: 123, nameWithOwner: 'acme/shop' }

test('lists workflows', () => {
  assert.deepEqual(buildListWorkflowsArgs('acme/shop'), [
    'workflow', 'list', '--repo', 'acme/shop', '--json', 'id,name,state,path', '--limit', '50',
  ])
})

test('lists runs with optional workflow/branch/status filters', () => {
  const base = buildListRunsArgs('acme/shop')
  assert.deepEqual(base, [
    'run', 'list', '--repo', 'acme/shop', '--limit', '30',
    '--json', 'databaseId,name,displayTitle,status,conclusion,headBranch,headSha,event,workflowName,createdAt,updatedAt',
  ])
  const filtered = buildListRunsArgs('acme/shop', { workflow: 'ci.yml', branch: 'main', status: 'failure' })
  assert.deepEqual(filtered.slice(-6), ['--workflow', 'ci.yml', '--branch', 'main', '--status', 'failure'])
})

test('views a run including its jobs/steps field', () => {
  assert.deepEqual(buildViewRunArgs(target), [
    'run', 'view', '123', '--repo', 'acme/shop',
    '--json', 'status,conclusion,jobs,headBranch,headSha,event,url,displayTitle,workflowName,createdAt,updatedAt',
  ])
})

test('run logs default to failed steps only, matching the troubleshooting use case', () => {
  assert.deepEqual(buildRunLogsArgs(target), ['run', 'view', '123', '--repo', 'acme/shop', '--log-failed'])
  assert.deepEqual(buildRunLogsArgs(target, { failedOnly: false }), ['run', 'view', '123', '--repo', 'acme/shop', '--log'])
})

test('rerun supports a failed-only mode', () => {
  assert.deepEqual(buildRerunArgs(target), ['run', 'rerun', '123', '--repo', 'acme/shop'])
  assert.deepEqual(buildRerunArgs(target, { failedOnly: true }), ['run', 'rerun', '123', '--repo', 'acme/shop', '--failed'])
})

test('cancel supports a force flag', () => {
  assert.deepEqual(buildCancelArgs(target), ['run', 'cancel', '123', '--repo', 'acme/shop'])
  assert.deepEqual(buildCancelArgs(target, { force: true }), ['run', 'cancel', '123', '--repo', 'acme/shop', '--force'])
})

test('triggering a workflow forwards ref and key=value inputs', () => {
  assert.deepEqual(
    buildTriggerWorkflowArgs('acme/shop', 'ci.yml', { ref: 'main', fields: [['environment', 'staging']] }),
    ['workflow', 'run', 'ci.yml', '--repo', 'acme/shop', '--ref', 'main', '-f', 'environment=staging'],
  )
})
