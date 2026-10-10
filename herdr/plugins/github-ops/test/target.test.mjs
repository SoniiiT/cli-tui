import assert from 'node:assert/strict'
import test from 'node:test'
import { delegatedPrompt, parseGithubTarget, slug, worktreeBranch } from '../lib/target.mjs'

test('normalizes GitHub issue and pull request links', () => {
  assert.deepEqual(parseGithubTarget('https://github.com/acme/shop/issues/42?tab=comments'), {
    owner: 'acme',
    repo: 'shop',
    nameWithOwner: 'acme/shop',
    kind: 'issue',
    number: 42,
    url: 'https://github.com/acme/shop/issues/42',
  })
  assert.equal(parseGithubTarget('https://github.com/acme/shop/pull/7/files#diff')?.kind, 'pull')
  assert.equal(parseGithubTarget('https://github.com/acme/shop/tree/main'), null)
  assert.equal(parseGithubTarget('https://example.com/acme/shop/issues/42'), null)
})

test('normalizes GitHub Actions run links', () => {
  assert.deepEqual(parseGithubTarget('https://github.com/acme/shop/actions/runs/123456?pr=7'), {
    owner: 'acme',
    repo: 'shop',
    nameWithOwner: 'acme/shop',
    kind: 'run',
    number: 123456,
    url: 'https://github.com/acme/shop/actions/runs/123456',
  })
})

test('builds safe, unique worktree branch names', () => {
  assert.equal(slug('  Résumé: fix / API  '), 'resume-fix-api')
  assert.equal(worktreeBranch({ kind: 'issue', number: 42 }, 'Fix flaky tests', 'abc123'), 'github-ops-issue-42-fix-flaky-tests-abc123')
  assert.equal(worktreeBranch({ kind: 'pull', number: 7 }, 'Refactor API', 'abc123', 'fix'), 'github-ops-pr-fix-7-refactor-api-abc123')
  assert.equal(worktreeBranch({ kind: 'run', number: 99 }, 'CI', 'abc123'), 'github-ops-run-99-ci-abc123')
})

test('delegation prompts forbid remote writes and keep the initial task reviewable', () => {
  const issue = delegatedPrompt({ kind: 'issue', number: 42, url: 'https://github.com/acme/shop/issues/42' }, 'Fix flaky tests')
  assert.match(issue, /implement the requested change/)
  assert.match(issue, /Do not commit, push, comment, close, approve, or merge/)

  const pull = delegatedPrompt({ kind: 'pull', number: 7, url: 'https://github.com/acme/shop/pull/7' }, 'Refactor API')
  assert.match(pull, /gh pr diff/)
  assert.match(pull, /Do not approve, comment, merge, push, or submit changes/)

  const pullFix = delegatedPrompt({ kind: 'pull', number: 7, url: 'https://github.com/acme/shop/pull/7' }, 'Refactor API', 'fix')
  assert.match(pullFix, /starts from the PR head/)
  assert.match(pullFix, /Do not commit, push, approve, comment, or merge/)

  const run = delegatedPrompt({
    kind: 'run', number: 99, nameWithOwner: 'acme/shop', workflowName: 'CI', headBranch: 'main', headSha: 'abc123', failedLog: 'Error: boom',
  }, 'CI')
  assert.match(run, /Error: boom/)
  assert.match(run, /Do not rerun, cancel, push, approve, or merge/)

  const release = delegatedPrompt({ kind: 'release', number: 'v1.2.3', nameWithOwner: 'acme/shop' }, 'v1.2.3')
  assert.match(release, /Draft release notes/)
  assert.match(release, /Do not create, edit, publish, or delete/)
})
