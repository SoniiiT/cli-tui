import { jsonCommand, run } from './process.mjs'

export function buildListTagsArgs(repo) {
  return ['api', `repos/${repo}/tags?per_page=100`]
}

export function listTags(repo, cwd) {
  return jsonCommand('gh', buildListTagsArgs(repo), { cwd })
}

export function buildResolveRefArgs(repo, ref) {
  return ['api', `repos/${repo}/commits/${encodeURIComponent(ref)}`, '--jq', '.sha']
}

export function resolveRef(repo, ref, cwd) {
  return run('gh', buildResolveRefArgs(repo, ref), { cwd }).stdout.trim()
}

export function buildCreateTagRefArgs(repo, name, sha) {
  return ['api', `repos/${repo}/git/refs`, '-f', `ref=refs/tags/${name}`, '-f', `sha=${sha}`]
}

export function createTagRef(repo, name, sha, cwd) {
  return run('gh', buildCreateTagRefArgs(repo, name, sha), { cwd })
}
