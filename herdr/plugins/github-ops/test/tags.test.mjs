import assert from 'node:assert/strict'
import test from 'node:test'
import { buildCreateTagRefArgs, buildListTagsArgs, buildResolveRefArgs } from '../lib/tags.mjs'

test('lists tags via the REST API, not the nonexistent gh tag subcommand', () => {
  assert.deepEqual(buildListTagsArgs('acme/shop'), ['api', 'repos/acme/shop/tags?per_page=100'])
})

test('resolves a ref to a commit SHA without mutating anything', () => {
  assert.deepEqual(buildResolveRefArgs('acme/shop', 'main'), ['api', 'repos/acme/shop/commits/main', '--jq', '.sha'])
})

test('creates a tag ref with an explicit sha, relying on gh api auto-POST for -f fields', () => {
  assert.deepEqual(buildCreateTagRefArgs('acme/shop', 'v1.2.3', 'abc123'), [
    'api', 'repos/acme/shop/git/refs', '-f', 'ref=refs/tags/v1.2.3', '-f', 'sha=abc123',
  ])
})
