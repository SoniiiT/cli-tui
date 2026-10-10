import assert from 'node:assert/strict'
import test from 'node:test'
import {
  buildCreateReleaseArgs,
  buildDeleteReleaseArgs,
  buildEditReleaseArgs,
  buildListReleasesArgs,
  buildPublishReleaseArgs,
  buildViewReleaseArgs,
} from '../lib/releases.mjs'

const target = { kind: 'release', number: 'v1.2.3', nameWithOwner: 'acme/shop' }

test('lists releases with the fields the board needs', () => {
  assert.deepEqual(buildListReleasesArgs('acme/shop'), [
    'release', 'list', '--repo', 'acme/shop', '--limit', '40',
    '--json', 'tagName,name,isDraft,isPrerelease,publishedAt,createdAt',
  ])
})

test('views a release by its tag', () => {
  assert.deepEqual(buildViewReleaseArgs(target), [
    'release', 'view', 'v1.2.3', '--repo', 'acme/shop',
    '--json', 'tagName,name,body,isDraft,isPrerelease,author,assets,publishedAt,url,targetCommitish',
  ])
})

test('generate-notes and manual notes are mutually exclusive', () => {
  const generated = buildCreateReleaseArgs({ tag: 'v1.2.3', repo: 'acme/shop', generateNotes: true, notes: 'ignored' })
  assert.ok(generated.includes('--generate-notes'))
  assert.equal(generated.includes('--notes'), false)

  const manual = buildCreateReleaseArgs({ tag: 'v1.2.3', repo: 'acme/shop', notes: 'Bugfixes' })
  assert.deepEqual(manual, ['release', 'create', 'v1.2.3', '--repo', 'acme/shop', '--notes', 'Bugfixes'])
})

test('draft and prerelease flags are appended only when requested', () => {
  const args = buildCreateReleaseArgs({ tag: 'v1.2.3', repo: 'acme/shop', title: 'First cut', draft: true, target: 'main' })
  assert.deepEqual(args, ['release', 'create', 'v1.2.3', '--repo', 'acme/shop', '--title', 'First cut', '--draft', '--target', 'main'])
})

test('editing a release uses --flag=value syntax for booleans, not a separate argument', () => {
  assert.deepEqual(buildEditReleaseArgs(target, { draft: false }), [
    'release', 'edit', 'v1.2.3', '--repo', 'acme/shop', '--draft=false',
  ])
  assert.deepEqual(buildPublishReleaseArgs(target), [
    'release', 'edit', 'v1.2.3', '--repo', 'acme/shop', '--draft=false',
  ])
})

test('deleting a release requires --yes to skip gh\'s interactive confirmation', () => {
  assert.deepEqual(buildDeleteReleaseArgs(target), ['release', 'delete', 'v1.2.3', '--repo', 'acme/shop', '--yes'])
  assert.deepEqual(buildDeleteReleaseArgs(target, { cleanupTag: true }), [
    'release', 'delete', 'v1.2.3', '--repo', 'acme/shop', '--yes', '--cleanup-tag',
  ])
})
