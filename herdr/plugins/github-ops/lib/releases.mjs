import { jsonCommand, run } from './process.mjs'

export function buildListReleasesArgs(repo) {
  return [
    'release', 'list', '--repo', repo, '--limit', '40',
    '--json', 'tagName,name,isDraft,isPrerelease,publishedAt,createdAt',
  ]
}

export function listReleases(repo, cwd) {
  return jsonCommand('gh', buildListReleasesArgs(repo), { cwd })
}

export function buildViewReleaseArgs(target) {
  return [
    'release', 'view', target.number, '--repo', target.nameWithOwner,
    '--json', 'tagName,name,body,isDraft,isPrerelease,author,assets,publishedAt,url,targetCommitish',
  ]
}

export function viewRelease(target, cwd) {
  return jsonCommand('gh', buildViewReleaseArgs(target), { cwd })
}

export function buildCreateReleaseArgs({ tag, repo, title, notes, generateNotes, draft, prerelease, target }) {
  const args = ['release', 'create', tag, '--repo', repo]
  if (title) args.push('--title', title)
  if (generateNotes) args.push('--generate-notes')
  else if (notes) args.push('--notes', notes)
  if (draft) args.push('--draft')
  if (prerelease) args.push('--prerelease')
  if (target) args.push('--target', target)
  return args
}

export function createRelease(options, cwd) {
  return run('gh', buildCreateReleaseArgs(options), { cwd })
}

export function buildEditReleaseArgs(target, { title, notes, draft, prerelease } = {}) {
  const args = ['release', 'edit', target.number, '--repo', target.nameWithOwner]
  if (title !== undefined) args.push('--title', title)
  if (notes !== undefined) args.push('--notes', notes)
  if (draft !== undefined) args.push(`--draft=${draft}`)
  if (prerelease !== undefined) args.push(`--prerelease=${prerelease}`)
  return args
}

export function editRelease(target, cwd, patch) {
  return run('gh', buildEditReleaseArgs(target, patch), { cwd })
}

export function buildPublishReleaseArgs(target) {
  return buildEditReleaseArgs(target, { draft: false })
}

export function publishRelease(target, cwd) {
  return run('gh', buildPublishReleaseArgs(target), { cwd })
}

export function buildDeleteReleaseArgs(target, { cleanupTag = false } = {}) {
  const args = ['release', 'delete', target.number, '--repo', target.nameWithOwner, '--yes']
  if (cleanupTag) args.push('--cleanup-tag')
  return args
}

export function deleteRelease(target, cwd, opts) {
  return run('gh', buildDeleteReleaseArgs(target, opts), { cwd })
}
