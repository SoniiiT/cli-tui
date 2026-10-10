import assert from 'node:assert/strict'
import test from 'node:test'
import { createWorktreeForAgent } from '../lib/delegation.mjs'

function dependencies() {
  const calls = []
  return {
    calls,
    run(command, args, options) {
      calls.push({ command, args, options })
      if (command === 'git' && args[0] === 'rev-parse') return { stdout: '/repos/shop\n' }
      if (command === 'git' && args[0] === 'remote') {
        return { stdout: 'origin https://github.com/acme/shop.git (fetch)\n' }
      }
      return { stdout: '' }
    },
    repository() {
      return { name: 'acme/shop', defaultBranch: 'main' }
    },
    createWorktree(options) {
      calls.push({ command: 'worktree', options })
      return { worktree: { path: options.path, branch: options.branch } }
    },
  }
}

const base = {
  title: 'Improve checkout validation',
  repoInfo: { name: 'acme/shop', defaultBranch: 'main' },
  cwd: '/repos/shop',
}

test('issue delegation uses the repository default branch', () => {
  const deps = dependencies()
  const target = { owner: 'acme', repo: 'shop', nameWithOwner: 'acme/shop', kind: 'issue', number: 42 }
  const result = createWorktreeForAgent({ ...base, target }, deps)
  const fetch = deps.calls.find((call) => call.command === 'git' && call.args[0] === 'fetch')
  const created = deps.calls.find((call) => call.command === 'worktree')
  assert.deepEqual(fetch.args.slice(-1), ['+refs/heads/main:refs/remotes/origin/main'])
  assert.equal(created.options.base, 'refs/remotes/origin/main')
  assert.equal(created.options.cwd, '/repos/shop')
  assert.equal(created.options.workspaceId, undefined)
  assert.equal(result.worktree.path, created.options.path)
})

test('PR follow-up worktrees start from the fetched pull-request head ref', () => {
  const deps = dependencies()
  const target = { owner: 'acme', repo: 'shop', nameWithOwner: 'acme/shop', kind: 'pull', number: 17 }
  const result = createWorktreeForAgent({ ...base, target, mode: 'fix' }, deps)
  const fetch = deps.calls.find((call) => call.command === 'git' && call.args[0] === 'fetch')
  const created = deps.calls.find((call) => call.command === 'worktree')
  assert.deepEqual(fetch.args.slice(-1), ['+refs/pull/17/head:refs/gh-ops/pull/17'])
  assert.equal(created.options.base, 'refs/gh-ops/pull/17')
  assert.equal(created.options.cwd, '/repos/shop')
  assert.match(created.options.branch, /github-ops-pr-fix-17-/)
  assert.equal(result.worktree.path, created.options.path)
})

test('a failing-run worktree starts from the run\'s commit, not the default branch', () => {
  const deps = dependencies()
  const target = { owner: 'acme', repo: 'shop', nameWithOwner: 'acme/shop', kind: 'run', number: 99, headSha: 'deadbeef' }
  const result = createWorktreeForAgent({ ...base, target }, deps)
  const fetch = deps.calls.find((call) => call.command === 'git' && call.args[0] === 'fetch')
  const created = deps.calls.find((call) => call.command === 'worktree')
  assert.deepEqual(fetch.args.slice(-1), ['+deadbeef:refs/gh-ops/run/99'])
  assert.equal(created.options.base, 'refs/gh-ops/run/99')
  assert.equal(result.worktree.path, created.options.path)
})

test('release-notes delegation falls back to the repository default branch', () => {
  const deps = dependencies()
  const target = { owner: 'acme', repo: 'shop', nameWithOwner: 'acme/shop', kind: 'release', number: 'v1.2.3' }
  createWorktreeForAgent({ ...base, target }, deps)
  const fetch = deps.calls.find((call) => call.command === 'git' && call.args[0] === 'fetch')
  assert.deepEqual(fetch.args.slice(-1), ['+refs/heads/main:refs/remotes/origin/main'])
})
