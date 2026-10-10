import assert from 'node:assert/strict'
import test from 'node:test'
import { buildMergeArgs, buildReviewArgs } from '../lib/github.mjs'

const pull = { kind: 'pull', number: 17, owner: 'acme', repo: 'shop', nameWithOwner: 'acme/shop' }

test('approval and request-changes are explicit review events', () => {
  assert.deepEqual(buildReviewArgs(pull, 'approve', 'Looks good'), [
    'pr', 'review', '17', '--repo', 'acme/shop', '--approve', '--body', 'Looks good',
  ])
  assert.deepEqual(buildReviewArgs(pull, 'request-changes', 'Fix the race'), [
    'pr', 'review', '17', '--repo', 'acme/shop', '--request-changes', '--body', 'Fix the race',
  ])
})

test('merge uses the selected method and explicitly disables auto-merge', () => {
  const args = buildMergeArgs(pull, 'squash')
  assert.deepEqual(args, [
    'pr', 'merge', '17', '--repo', 'acme/shop', '--squash', '--disable-auto',
  ])
  assert.equal(args.includes('--auto'), false)
  assert.equal(args.includes('--admin'), false)
})
