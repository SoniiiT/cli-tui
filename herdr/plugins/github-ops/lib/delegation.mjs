import { mkdirSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { repository } from './github.mjs'
import { run } from './process.mjs'
import { worktreeBranch } from './target.mjs'

function matchesRemote(url, repo) {
  const escaped = repo.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  return new RegExp(`(?:github\\.com[:/])${escaped}(?:\\.git)?$`, 'i').test(url.trim())
}

function repositoryRemote(root, repo, runCommand = run) {
  const remotes = runCommand('git', ['remote', '-v'], { cwd: root }).stdout
  for (const line of remotes.split('\n')) {
    const match = /^([^\s]+)\s+([^\s]+)\s+\(fetch\)$/.exec(line.trim())
    if (match && matchesRemote(match[2], repo)) return match[1]
  }
  throw new Error(`No local fetch remote for ${repo} was found in ${root}. Open GitHub Ops from that repository's Herdr workspace to delegate.`)
}

export function worktreePathFor(repoName, branch, home = os.homedir()) {
  const repoSlug = repoName.toLowerCase().replace(/[^a-z0-9._-]+/g, '-')
  return path.join(home, '.herdr', 'worktrees', repoSlug, branch)
}

export function gitWorktreeAddArgs({ branch, path: checkoutPath, base }) {
  return ['worktree', 'add', '-b', branch, checkoutPath, base]
}

export function createWorktreeForAgent(
  { target, title, repoInfo, cwd, mode = 'work' },
  dependencies = {},
) {
  const runCommand = dependencies.run ?? run
  const getRepository = dependencies.repository ?? repository
  const root = runCommand('git', ['rev-parse', '--show-toplevel'], { cwd }).stdout.trim()
  if (!root) throw new Error('Could not find a local Git checkout for agent delegation.')
  const localRepo = getRepository(root)
  if (localRepo.name.toLowerCase() !== repoInfo.name.toLowerCase()) {
    throw new Error(`The selected item belongs to ${repoInfo.name}, but the current checkout is ${localRepo.name}. Open GitHub Ops from a local checkout of the target repository to delegate it.`)
  }

  const remote = repositoryRemote(root, repoInfo.name, runCommand)
  let baseRef = `refs/remotes/${remote}/${repoInfo.defaultBranch}`
  if (target.kind === 'pull' && mode === 'fix') {
    baseRef = `refs/gh-ops/pull/${target.number}`
    runCommand('git', ['fetch', '--quiet', remote, `+refs/pull/${target.number}/head:${baseRef}`], { cwd: root })
  } else if (target.kind === 'run') {
    baseRef = `refs/gh-ops/run/${target.number}`
    runCommand('git', ['fetch', '--quiet', remote, `+${target.headSha}:${baseRef}`], { cwd: root })
  } else {
    runCommand('git', ['fetch', '--quiet', remote, `+refs/heads/${repoInfo.defaultBranch}:${baseRef}`], { cwd: root })
  }

  const branch = worktreeBranch(target, title, Date.now().toString(36), mode)
  const kindLabel = { pull: 'PR', issue: 'Issue', run: 'Run', release: 'Release' }[target.kind] ?? target.kind
  const label = `${kindLabel} #${target.number}: ${title}`.slice(0, 90)
  const checkoutPath = worktreePathFor(repoInfo.name, branch, dependencies.home)
  const options = {
    cwd: root,
    path: checkoutPath,
    repoInfo,
    branch,
    base: baseRef,
    label,
  }
  if (dependencies.createWorktree) return dependencies.createWorktree(options)

  const makeDirectory = dependencies.mkdirSync ?? mkdirSync
  makeDirectory(path.dirname(checkoutPath), { recursive: true })
  runCommand('git', gitWorktreeAddArgs(options), { cwd: root })
  return { worktree: { path: checkoutPath, branch, label } }
}
