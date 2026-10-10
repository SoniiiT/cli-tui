import { run } from './process.mjs'

const herdrBin = () => process.env.HERDR_BIN_PATH || 'herdr'

function unwrap(output) {
  for (const line of `${output.stdout}\n${output.stderr}`.split('\n')) {
    const text = line.trim()
    if (!text.startsWith('{')) continue
    try {
      const value = JSON.parse(text)
      if (value.error) throw new Error(value.error.message || 'Herdr command failed')
      if ('result' in value) return value.result
    } catch (error) {
      if (error instanceof SyntaxError) continue
      throw error
    }
  }
  throw new Error('Herdr returned no JSON result')
}

export function listAgentKinds() {
  const output = run(herdrBin(), ['agent', 'start', '--help'], { allowFailure: true })
  const text = `${output.stdout}\n${output.stderr}`
  const values = /\[possible values: ([^\]]+)\]/i.exec(text)?.[1]
  return values ? values.split(',').map((item) => item.trim()).filter(Boolean) : ['opencode']
}

export function listAgents() {
  return unwrap(run(herdrBin(), ['agent', 'list']))?.agents ?? []
}

export function getAgent(target) {
  return unwrap(run(herdrBin(), ['agent', 'get', target]))?.agent ?? {}
}

export function focusWorkspaceArgs(workspaceId) {
  return ['workspace', 'focus', workspaceId]
}

export function focusWorkspace(workspaceId) {
  return unwrap(run(herdrBin(), focusWorkspaceArgs(workspaceId)))
}

export function openAgentPopupArgs({ pluginId, cwd }) {
  return [
    'plugin', 'pane', 'open', '--plugin', pluginId, '--entrypoint', 'agent', '--placement', 'zoomed',
    '--cwd', cwd, '--focus',
  ]
}

export function openAgentPopup({ pluginId, workspaceId, cwd }) {
  if (workspaceId) focusWorkspace(workspaceId)
  return unwrap(run(herdrBin(), openAgentPopupArgs({ pluginId, cwd }), { cwd }))?.plugin_pane?.pane ?? {}
}

export function startAgent({ paneId, name, kind }) {
  return unwrap(run(herdrBin(), ['agent', 'start', name, '--kind', kind, '--pane', paneId, '--timeout', '90000']))
}

export function focusAgent(target) {
  return unwrap(run(herdrBin(), ['agent', 'focus', target]))
}

export function paneSendTextArgs(paneId, text) {
  return ['pane', 'send-text', paneId, text]
}

export function typePrompt(paneId, prompt) {
  // `pane send-text` succeeds with an empty response body; run() already checks its exit status.
  return run(herdrBin(), paneSendTextArgs(paneId, prompt))
}

export function promptAgent(target, prompt) {
  return unwrap(run(herdrBin(), ['agent', 'prompt', target, prompt]))
}
