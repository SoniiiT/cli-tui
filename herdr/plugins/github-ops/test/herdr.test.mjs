import assert from 'node:assert/strict'
import test from 'node:test'
import { gitWorktreeAddArgs, worktreePathFor } from '../lib/delegation.mjs'
import { focusWorkspaceArgs, openAgentPopupArgs, paneSendTextArgs } from '../lib/herdr.mjs'

test('creates a linked worktree without adding a Herdr workspace', () => {
  const path = worktreePathFor('SoniiiT/docker', 'github-ops-pr-49-fix', '/home/test')
  assert.equal(path, '/home/test/.herdr/worktrees/soniiit-docker/github-ops-pr-49-fix')

  const args = gitWorktreeAddArgs({
    branch: 'github-ops-pr-49-fix',
    base: 'refs/gh-ops/pull/49',
    path,
  })

  assert.deepEqual(args, [
    'worktree', 'add', '-b', 'github-ops-pr-49-fix', path, 'refs/gh-ops/pull/49',
  ])
})

test('opens a focused overlay popup in the active Herdr workspace at the worktree path', () => {
  assert.deepEqual(focusWorkspaceArgs('w9'), ['workspace', 'focus', 'w9'])
  assert.deepEqual(openAgentPopupArgs({
    pluginId: 'soniiit.github-ops',
    cwd: '/worktrees/docker-pr-49',
  }), [
    'plugin', 'pane', 'open', '--plugin', 'soniiit.github-ops', '--entrypoint', 'agent',
    '--placement', 'zoomed', '--cwd', '/worktrees/docker-pr-49', '--focus',
  ])
  assert.equal(openAgentPopupArgs({ pluginId: 'soniiit.github-ops', cwd: '/worktrees/docker-pr-49' }).includes('--workspace'), false)
})

test('sends the reviewed task into the popup without requiring a JSON response', () => {
  assert.deepEqual(paneSendTextArgs('wH:p8', 'startup check'), [
    'pane', 'send-text', 'wH:p8', 'startup check',
  ])
})
