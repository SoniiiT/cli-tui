export function readContext(env = process.env) {
  try {
    const value = JSON.parse(env.HERDR_PLUGIN_CONTEXT_JSON || '{}')
    return value && typeof value === 'object' && !Array.isArray(value) ? value : {}
  } catch {
    return {}
  }
}

export function pluginContext(env = process.env) {
  const context = readContext(env)
  return {
    cwd: env.GH_OPS_CWD || context.focused_pane_cwd || context.workspace_cwd || process.cwd(),
    workspaceId: env.GH_OPS_WORKSPACE_ID || context.workspace_id || '',
    paneId: env.HERDR_PANE_ID || context.focused_pane_id || '',
    sourcePaneId: env.GH_OPS_ORIGIN_PANE || context.focused_pane_id || '',
    focusedAgent: env.GH_OPS_AGENT || context.focused_pane_agent || '',
    url: env.GH_OPS_URL || env.HERDR_PLUGIN_CLICKED_URL || context.clicked_url || '',
    mode: env.GH_OPS_MODE || 'browse',
  }
}
