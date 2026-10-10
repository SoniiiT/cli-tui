#!/usr/bin/env node
import { spawnSync } from 'node:child_process'
import { pluginContext } from '../lib/context.mjs'

const pluginId = 'soniiit.github-ops'
const mode = process.argv[2] || 'browse'
const context = pluginContext()
const herdr = process.env.HERDR_BIN_PATH || 'herdr'
const args = [
  'plugin', 'pane', 'open', '--plugin', pluginId, '--entrypoint', 'ops',
  '--placement', 'zoomed', '--focus',
]

for (const [key, value] of Object.entries({
  GH_OPS_MODE: mode,
  GH_OPS_CWD: context.cwd,
  GH_OPS_WORKSPACE_ID: context.workspaceId,
  GH_OPS_ORIGIN_PANE: context.focusedPaneId,
  GH_OPS_AGENT: context.focusedAgent,
  GH_OPS_URL: context.url,
})) {
  if (value) args.push('--env', `${key}=${value}`)
}

const result = spawnSync(herdr, args, { encoding: 'utf8' })
if (result.error) {
  process.stderr.write(`GitHub Ops: could not run Herdr: ${result.error.message}\n`)
  process.exit(1)
}
if (result.stdout) process.stdout.write(result.stdout)
if (result.stderr) process.stderr.write(result.stderr)
process.exit(result.status ?? 1)
