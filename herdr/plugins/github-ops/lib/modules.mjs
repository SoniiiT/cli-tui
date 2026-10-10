import { listIssues, listPullRequests, viewTarget } from './github.mjs'
import { listTags } from './tags.mjs'
import { listReleases, viewRelease } from './releases.mjs'
import { listRuns, viewRun } from './workflow-runs.mjs'

function ownerRepo(nameWithOwner) {
  const [owner, repo] = nameWithOwner.split('/')
  return { owner, repo }
}

// Each source module owns its list query and detail tabs. New GitHub resource types can register here.
// `toTarget` is only needed when a module's natural identifier isn't a GitHub issue/pull URL
// (parseGithubTarget's default path); `number` is overloaded to carry each kind's own id
// (tag name, release tagName, or numeric run databaseId) so the existing selection/cache/
// delegation code (keyed on target.number) needs no changes for the new kinds.
export const MODULES = [
  {
    id: 'issues',
    label: 'Issues',
    kind: 'issue',
    detailTabs: ['Overview', 'Description', 'Comments', 'Related'],
    load: (repo, cwd, state) => listIssues(repo, cwd, state).map((item) => ({ ...item, kind: 'issue' })),
    detail: (target, cwd) => viewTarget(target, cwd),
  },
  {
    id: 'pulls',
    label: 'Pull Requests',
    kind: 'pull',
    detailTabs: ['Overview', 'Description', 'Comments', 'Reviews', 'Files', 'Checks'],
    load: (repo, cwd, state) => listPullRequests(repo, cwd, state).map((item) => ({ ...item, kind: 'pull' })),
    detail: (target, cwd) => viewTarget(target, cwd),
  },
  {
    id: 'tags',
    label: 'Tags',
    kind: 'tag',
    detailTabs: ['Overview', 'Commit'],
    load: (repo, cwd) => listTags(repo, cwd).map((tag) => ({
      number: tag.name, title: tag.name, sha: tag.commit?.sha, kind: 'tag', nameWithOwner: repo,
    })),
    toTarget: (item) => ({ kind: 'tag', number: item.number, sha: item.sha, nameWithOwner: item.nameWithOwner, ...ownerRepo(item.nameWithOwner) }),
    detail: (target) => ({ ...target }),
  },
  {
    id: 'releases',
    label: 'Releases',
    kind: 'release',
    detailTabs: ['Overview', 'Notes', 'Assets'],
    load: (repo, cwd) => listReleases(repo, cwd).map((release) => ({
      ...release, number: release.tagName, title: release.name || release.tagName, kind: 'release', nameWithOwner: repo,
    })),
    toTarget: (item) => ({ kind: 'release', number: item.number, nameWithOwner: item.nameWithOwner, ...ownerRepo(item.nameWithOwner) }),
    detail: (target, cwd) => viewRelease(target, cwd),
  },
  {
    id: 'actions',
    label: 'Actions',
    kind: 'run',
    detailTabs: ['Overview', 'Jobs'],
    load: (repo, cwd) => listRuns(repo, cwd).map((run) => ({
      ...run, number: run.databaseId, title: run.displayTitle || run.name, kind: 'run', nameWithOwner: repo,
    })),
    toTarget: (item) => ({
      kind: 'run', number: item.number, nameWithOwner: item.nameWithOwner,
      headSha: item.headSha, headBranch: item.headBranch, workflowName: item.workflowName,
      ...ownerRepo(item.nameWithOwner),
    }),
    detail: (target, cwd) => viewRun(target, cwd),
  },
]
