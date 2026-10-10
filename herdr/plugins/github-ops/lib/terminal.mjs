const RESET = '\u001b[0m'
const COLORS = {
  accent: '\u001b[38;2;121;192;255m',
  bright: '\u001b[38;2;230;237;243m',
  border: '\u001b[38;2;65;83;106m',
  cyan: '\u001b[38;2;86;212;220m',
  green: '\u001b[38;2;126;231;135m',
  muted: '\u001b[38;2;139;148;158m',
  red: '\u001b[38;2;255;123;114m',
  yellow: '\u001b[38;2;227;179;65m',
  bold: '\u001b[1m',
}
const SAFE_STYLE_SEQUENCES = new Set([RESET, ...Object.values(COLORS)])
const graphemeSegmenter = new Intl.Segmenter(undefined, { granularity: 'grapheme' })

export function sanitizeTerminalText(value) {
  const source = String(value ?? '')
    .replace(/\r\n?/g, '\n')
    .replace(/\t/g, '    ')
    .replace(/\u001b(?:\[[0-?]*[ -/]*[@-~])?/g, (sequence) => SAFE_STYLE_SEQUENCES.has(sequence) ? sequence : '')
  return source.replace(/[\u0000-\u001a\u001c-\u001f\u007f-\u009f]/g, '')
}

function paint(value, ...styles) {
  return `${styles.join('')}${value}${RESET}`
}

function graphemes(value) {
  return Array.from(graphemeSegmenter.segment(String(value ?? '')), (entry) => entry.segment)
}

function isZeroWidth(codePoint) {
  const character = String.fromCodePoint(codePoint)
  return codePoint === 0x200d
    || (codePoint >= 0xfe00 && codePoint <= 0xfe0f)
    || (codePoint >= 0xe0100 && codePoint <= 0xe01ef)
    || (codePoint >= 0x1f3fb && codePoint <= 0x1f3ff)
    || /\p{Mark}/u.test(character)
}

function isWide(codePoint) {
  return codePoint >= 0x1100 && (
    codePoint <= 0x115f
    || codePoint === 0x2329 || codePoint === 0x232a
    || (codePoint >= 0x2e80 && codePoint <= 0xa4cf)
    || (codePoint >= 0xac00 && codePoint <= 0xd7a3)
    || (codePoint >= 0xf900 && codePoint <= 0xfaff)
    || (codePoint >= 0xfe10 && codePoint <= 0xfe19)
    || (codePoint >= 0xfe30 && codePoint <= 0xfe6f)
    || (codePoint >= 0xff00 && codePoint <= 0xff60)
    || (codePoint >= 0xffe0 && codePoint <= 0xffe6)
    || (codePoint >= 0x1f300 && codePoint <= 0x1faff)
    || (codePoint >= 0x20000 && codePoint <= 0x3fffd)
  )
}

function graphemeWidth(grapheme) {
  let width = 0
  for (const character of grapheme) {
    const codePoint = character.codePointAt(0)
    if (!isZeroWidth(codePoint)) width = Math.max(width, isWide(codePoint) ? 2 : 1)
  }
  return width
}

export function displayWidth(value) {
  const plain = String(value ?? '').replace(/\u001b\[[0-?]*[ -/]*[@-~]/g, '')
  return graphemes(plain).reduce((width, grapheme) => width + graphemeWidth(grapheme), 0)
}

function fit(value, width) {
  const source = sanitizeTerminalText(value)
  const widthNow = displayWidth(source)
  if (widthNow <= width) return `${source}${' '.repeat(width - widthNow)}`

  const plain = source.replace(/\u001b\[[0-?]*[ -/]*[@-~]/g, '')
  const result = []
  let used = 0
  for (const grapheme of graphemes(plain)) {
    const nextWidth = graphemeWidth(grapheme)
    if (used + nextWidth > width - 1) break
    result.push(grapheme)
    used += nextWidth
  }
  return `${result.join('')}…`
}

export function wrap(value, width) {
  const result = []
  for (const raw of sanitizeTerminalText(value).split(/\r?\n/)) {
    if (!raw) {
      result.push('')
      continue
    }
    let line = ''
    for (const word of raw.split(/\s+/)) {
      if (!line) line = word
      else if (displayWidth(`${line} ${word}`) <= width) line += ` ${word}`
      else {
        result.push(...splitLong(line, width))
        line = word
      }
    }
    result.push(...splitLong(line, width))
  }
  return result
}

function splitLong(value, width) {
  const lines = []
  let line = ''
  let lineWidth = 0
  for (const grapheme of graphemes(value)) {
    const nextWidth = graphemeWidth(grapheme)
    if (line && lineWidth + nextWidth > width) {
      lines.push(line)
      line = ''
      lineWidth = 0
    }
    line += grapheme
    lineWidth += nextWidth
  }
  lines.push(line)
  return lines
}

function panel(title, content, width, height) {
  const inner = Math.max(4, width - 4)
  const topLabel = ` ${title} `
  const top = `${COLORS.border}╭${RESET}${paint(topLabel, COLORS.accent, COLORS.bold)}${COLORS.border}${'─'.repeat(Math.max(0, width - topLabel.length - 2))}╮${RESET}`
  const body = content.slice(0, Math.max(0, height - 2)).map((line) => `${COLORS.border}│${RESET} ${fit(line, inner)} ${COLORS.border}│${RESET}`)
  while (body.length < height - 2) body.push(`${COLORS.border}│${RESET} ${' '.repeat(inner)} ${COLORS.border}│${RESET}`)
  return [top, ...body, `${COLORS.border}╰${'─'.repeat(Math.max(0, width - 2))}╯${RESET}`]
}

function joinPanels(left, right, leftWidth, rightWidth) {
  const height = Math.max(left.length, right.length)
  const rows = []
  for (let index = 0; index < height; index++) {
    rows.push(`${fit(left[index] ?? '', leftWidth)} ${fit(right[index] ?? '', rightWidth)}`)
  }
  return rows
}

function detailLines(module, item, detail, tab, width) {
  if (!item) return ['Choose an item on the left.']
  if (!detail) return [`#${item.number} ${item.title}`, '', 'Press Enter to load full details.']
  if (tab === 'Description') return wrap(detail.body?.trim() || '(no description)', width)
  if (tab === 'Comments') {
    const entries = detail.comments ?? []
    return entries.length
      ? entries.flatMap((comment) => wrap(`@${comment.author?.login ?? 'unknown'} · ${comment.body ?? ''}`, width).concat(''))
      : ['No comments yet.']
  }
  if (tab === 'Reviews') {
    const entries = detail.reviews ?? []
    return entries.length
      ? entries.flatMap((review) => wrap(`${review.state ?? 'REVIEW'} · @${review.author?.login ?? 'unknown'} · ${review.body ?? ''}`, width).concat(''))
      : ['No reviews yet.']
  }
  if (tab === 'Files') {
    const files = detail.files ?? []
    return files.length
      ? files.map((file) => `${file.path ?? file.name ?? 'file'}  +${file.additions ?? 0} / -${file.deletions ?? 0}`)
      : ['No changed-file details available. Press V for the full GHZinga diff view.']
  }
  if (tab === 'Checks') {
    const checks = detail.statusCheckRollup ?? []
    return checks.length
      ? checks.map((check) => `${check.name ?? check.workflowName ?? check.__typename ?? 'check'} · ${check.conclusion ?? check.state ?? check.status ?? 'pending'}`)
      : ['No check details available.']
  }
  if (tab === 'Related') {
    const related = detail.closedByPullRequestsReferences ?? detail.projectItems ?? []
    return related.length
      ? related.map((entry) => `#${entry.number ?? ''} ${entry.title ?? entry.project?.title ?? ''}`)
      : ['No related work shown.']
  }
  if (tab === 'Commit') {
    return [`Tag: ${detail.number}`, `Commit: ${detail.sha ?? 'unknown'}`]
  }
  if (tab === 'Notes') {
    return wrap(detail.body?.trim() || '(no release notes)', width)
  }
  if (tab === 'Assets') {
    const assets = detail.assets ?? []
    return assets.length
      ? assets.map((asset) => `${asset.name ?? 'asset'}  ${asset.size ? `${Math.round(asset.size / 1024)} KB` : ''}`)
      : ['No uploaded assets.']
  }
  if (tab === 'Jobs') {
    const jobs = detail.jobs ?? []
    if (!jobs.length) return ['No job details available.']
    return jobs.flatMap((job) => [
      `${job.conclusion || job.status || 'pending'} · ${job.name}`,
      ...(job.steps ?? []).map((step) => `   ${step.conclusion || step.status || 'pending'} · ${step.number}. ${step.name}`),
      '',
    ])
  }

  if (module.kind === 'tag') {
    return [`Tag ${detail.number}`, `Commit: ${detail.sha ?? 'unknown'}`, '', 'Click a detail tab or use [ and ].']
  }
  if (module.kind === 'release') {
    const lines = [
      `Release ${detail.number}: ${detail.title ?? detail.name ?? detail.number}`,
      `Status: ${detail.isDraft ? 'DRAFT' : detail.isPrerelease ? 'PRERELEASE' : 'PUBLISHED'}`,
    ]
    if (detail.author?.login) lines.push(`Author: @${detail.author.login}`)
    if (detail.publishedAt) lines.push(`Published: ${detail.publishedAt}`)
    lines.push('', 'Click a detail tab or use [ and ].')
    return lines
  }
  if (module.kind === 'run') {
    const lines = [
      `Run #${detail.number}: ${detail.displayTitle ?? detail.title ?? ''}`,
      `Status: ${detail.conclusion || detail.status || 'unknown'} · Workflow: ${detail.workflowName ?? '?'}`,
      `Branch: ${detail.headBranch ?? '?'} · Event: ${detail.event ?? '?'}`,
    ]
    lines.push('', 'Click a detail tab or use [ and ].')
    return lines
  }

  const lines = [
    `${module.kind === 'pull' ? 'Pull request' : 'Issue'} #${detail.number}: ${detail.title}`,
    `State: ${detail.state ?? 'OPEN'} · Author: @${detail.author?.login ?? 'unknown'}`,
  ]
  if (detail.labels?.length) lines.push(`Labels: ${detail.labels.map((label) => label.name).join(', ')}`)
  if (detail.assignees?.length) lines.push(`Assignees: ${detail.assignees.map((assignee) => assignee.login).join(', ')}`)
  if (module.kind === 'pull') {
    lines.push(`Review: ${detail.reviewDecision || 'not reviewed'} · Merge: ${detail.mergeStateStatus ?? 'unknown'}`)
    lines.push(`${detail.headRefName ?? '?'} → ${detail.baseRefName ?? '?'}`)
    if (detail.isDraft) lines.push('Draft pull request')
  } else {
    lines.push(`Updated: ${detail.updatedAt ?? 'unknown'}`)
  }
  lines.push('', 'Click a detail tab or use [ and ].')
  return lines
}

export function toolbarItems(module) {
  const base = [
    { key: 'O', label: 'Repo', type: 'action', action: 'repo' },
    { key: 'R', label: 'Refresh', type: 'action', action: 'refresh' },
  ]
  const quit = { key: 'q', label: 'Quit', type: 'action', action: 'q' }

  if (module.kind === 'tag') {
    return [...base, { key: 'n', label: 'New Tag', type: 'action', action: 'tag-new' }, quit]
  }
  if (module.kind === 'release') {
    return [
      ...base,
      { key: 'n', label: 'New Release', type: 'action', action: 'release-new' },
      { key: 'p', label: 'Publish', type: 'action', action: 'release-publish' },
      { key: 'x', label: 'Delete', type: 'action', action: 'release-delete' },
      quit,
    ]
  }
  if (module.kind === 'run') {
    return [
      ...base,
      { key: 'n', label: 'Trigger', type: 'action', action: 'run-trigger' },
      { key: 'l', label: 'Logs', type: 'action', action: 'run-logs' },
      { key: 'u', label: 'Rerun', type: 'action', action: 'run-rerun' },
      { key: 'f', label: 'Rerun Failed', type: 'action', action: 'run-rerun-failed' },
      { key: 'x', label: 'Cancel', type: 'action', action: 'run-cancel' },
      { key: 'd', label: 'Delegate', type: 'action', action: 'd' },
      quit,
    ]
  }

  // issue / pull (unchanged)
  const items = [
    ...base,
    { key: 'C', label: 'Open/All', type: 'action', action: 'state' },
    { key: 'v', label: 'View', type: 'action', action: 'v' },
    { key: 'c', label: 'Comment', type: 'action', action: 'c' },
    { key: 'e', label: 'Edit', type: 'action', action: 'e' },
    { key: 'd', label: 'Delegate', type: 'action', action: 'd' },
  ]
  if (module.kind === 'pull') {
    items.push(
      { key: 'a', label: 'Approve', type: 'action', action: 'a' },
      { key: 'r', label: 'Changes', type: 'action', action: 'r' },
      { key: 'm', label: 'Merge', type: 'action', action: 'm' },
    )
  }
  items.push({ key: 'x', label: 'Close', type: 'action', action: 'x' }, quit)
  return items
}

function layoutToolbar(items, width) {
  const lines = ['']
  const hits = []
  let row = 0
  let column = 0

  for (const item of items) {
    const label = `[${item.key}] ${item.label}`
    const gap = column ? 2 : 0
    if (column && column + gap + Array.from(label).length > width) {
      row++
      column = 0
      lines.push('')
    }
    const start = column + (column ? 2 : 0)
    lines[row] += `${column ? '  ' : ''}${label}`
    hits.push({ ...item, row, start, end: start + Array.from(label).length })
    column = start + Array.from(label).length
  }

  return { lines, hits }
}

function layoutTabs(module, selectedTab, width, startX, startY) {
  const lines = ['']
  const hits = []
  let row = 0
  let column = 0

  for (let index = 0; index < module.detailTabs.length; index++) {
    const tab = module.detailTabs[index]
    const label = tab === selectedTab ? `[${tab}]` : tab
    const separator = column ? '  |  ' : ''
    const labelWidth = Array.from(label).length
    if (column && column + Array.from(separator).length + labelWidth > width) {
      row++
      column = 0
      lines.push('')
    }
    const gap = column ? Array.from(separator).length : 0
    const shown = fit(label, width - column - gap).trimEnd()
    const start = column + gap
    lines[row] += `${column ? separator : ''}${shown}`
    hits.push({ type: 'detail-tab', index, tab, row: startY + row, start: startX + start, end: startX + start + Array.from(shown).length })
    column = start + Array.from(shown).length
  }

  return { lines, hits }
}

function boardMetrics(state, width, height) {
  const normalizedWidth = Math.max(72, width || 100)
  const normalizedHeight = Math.max(20, height || 30)
  const module = state.module
  const toolbar = layoutToolbar(toolbarItems(module), normalizedWidth)
  const help = wrap('Click list rows, module tabs, detail tabs and action buttons · ↑↓/j k select · Tab/1-5 modules · [ ] detail tabs', normalizedWidth)
  const bodyHeight = Math.max(4, normalizedHeight - 3 - help.length - toolbar.lines.length - 1)
  const leftWidth = Math.max(30, Math.floor((normalizedWidth - 1) * 0.43))
  const rightWidth = normalizedWidth - leftWidth - 1
  const tabs = layoutTabs(module, state.detailTab, Math.max(8, rightWidth - 5), leftWidth + 3, 4)
  return {
    width: normalizedWidth,
    height: normalizedHeight,
    leftWidth,
    rightWidth,
    bodyHeight,
    help,
    toolbar,
    toolbarTop: 3 + bodyHeight + help.length,
    tabs,
  }
}

function moduleHits(state) {
  let column = 2
  return state.modules.map((module, index) => {
    const label = module === state.module ? `[${module.label}]` : module.label
    const hit = { type: 'module', index, start: column, end: column + Array.from(label).length, row: 1 }
    column += Array.from(label).length + 3
    return hit
  })
}

function modalHit(state, x, y, width, height) {
  const modal = state.modal
  const contentWidth = Math.min(width - 8, 88)
  const body = modal.lines.slice(0, Math.max(3, height - 8))
  const top = Math.max(3, Math.floor((height - (body.length + 2)) / 2))
  const left = Math.max(2, Math.floor((width - contentWidth) / 2))
  if (x < left || x >= left + contentWidth || y < top + 1 || y >= top + 1 + body.length) return null

  const lineIndex = y - top - 1
  if (modal.dropdownToggleLine === lineIndex) return { type: 'modal-toggle' }
  if (modal.type === 'form') {
    const fieldIndex = modal.lineFieldMap?.[lineIndex - modal.fieldStart]
    if (fieldIndex !== undefined) return { type: 'modal-field', index: fieldIndex }
  }
  const choices = modal.visibleChoices ?? modal.options ?? modal.candidates ?? []
  const choiceStart = modal.choiceStart ?? (modal.type === 'delegate' ? 2 : 1)
  const buttonLineIndex = modal.buttonLineIndex ?? choiceStart + choices.length
  if (lineIndex >= choiceStart && lineIndex < choiceStart + choices.length) {
    return { type: 'modal-choice', index: (modal.choiceOffset ?? 0) + lineIndex - choiceStart }
  }
  if (lineIndex === buttonLineIndex) {
    let buttonStart = left + 2
    if (modal.defaultButton) {
      const defaultChoice = `[${modal.defaultButton}]`
      if (x >= buttonStart && x < buttonStart + defaultChoice.length) return { type: 'modal-default' }
      buttonStart += defaultChoice.length + 3
    }
    const primary = `[${modal.primaryButton ?? 'Continue'}]`
    const cancel = '[Cancel]'
    if (x >= buttonStart && x < buttonStart + primary.length) return { type: 'modal-primary' }
    const cancelStart = buttonStart + primary.length + 3
    if (x >= cancelStart && x < cancelStart + cancel.length) return { type: 'modal-cancel' }
  }
  return null
}

export function hitTestBoard(state, x, y, columns, rows) {
  const width = Math.max(72, columns || 100)
  const height = Math.max(20, rows || 30)
  if (state.modal) return modalHit(state, x, y, width, height)

  const layout = boardMetrics(state, width, height)
  if (y === 1) {
    const hit = moduleHits(state).find((entry) => x >= entry.start && x < entry.end)
    if (hit) return hit
  }
  if (x < layout.leftWidth && y >= 5) {
    const index = y - 5
    if (index < state.filteredItems.length && index < layout.bodyHeight - 3) return { type: 'item', index }
  }
  const tab = layout.tabs.hits.find((entry) => y === entry.row && x >= entry.start && x < entry.end)
  if (tab) return tab

  const toolbarRow = y - layout.toolbarTop
  const action = layout.toolbar.hits.find((entry) => entry.row === toolbarRow && x >= entry.start && x < entry.end)
  if (action) return action
  return null
}

function listHeader(kind) {
  const labels = {
    tag: '#   TAG                              COMMIT',
    release: '#   RELEASE                     STATUS           TITLE',
    run: '#        STATUS         WORKFLOW                  BRANCH',
  }
  return paint(labels[kind] ?? `#   ${kind === 'pull' ? 'PR' : 'ISSUE'}   STATE / REVIEW   TITLE`, COLORS.muted, COLORS.bold)
}

function listRow(kind, item, leftWidth) {
  if (kind === 'tag') {
    return `${fit(item.title, Math.max(8, leftWidth - 14))} ${paint((item.sha ?? '').slice(0, 7), COLORS.muted)}`
  }
  if (kind === 'release') {
    const status = item.isDraft ? paint('DRAFT', COLORS.yellow, COLORS.bold) : item.isPrerelease ? paint('PRERELEASE', COLORS.yellow) : paint('PUBLISHED', COLORS.green)
    return `${fit(item.number, 24)} ${fit(status, 14)}  ${fit(item.title, Math.max(8, leftWidth - 44))}`
  }
  if (kind === 'run') {
    const conclusion = item.conclusion || item.status || 'pending'
    const style = conclusion === 'success' || conclusion === 'completed' ? COLORS.green : conclusion === 'failure' ? COLORS.red : COLORS.yellow
    return `${String(item.number).padStart(10)}  ${fit(paint(conclusion, style, COLORS.bold), 22)} ${fit(item.workflowName ?? '', 24)}  ${fit(item.headBranch ?? '', Math.max(6, leftWidth - 64))}`
  }
  const author = item.author?.login ? paint(` @${item.author.login}`, COLORS.muted) : ''
  const itemState = item.state ?? 'OPEN'
  const stateStyle = itemState === 'OPEN' ? COLORS.green : itemState === 'MERGED' ? COLORS.cyan : COLORS.muted
  const status = item.reviewDecision
    ? `${paint(itemState, stateStyle, COLORS.bold)}/${paint(item.reviewDecision, item.reviewDecision === 'APPROVED' ? COLORS.green : COLORS.yellow)}`
    : paint(itemState, stateStyle, COLORS.bold)
  return `${String(item.number).padStart(4)}  ${fit(status, 18)}  ${fit(item.title, Math.max(8, leftWidth - 31))} ${author}`
}

export function renderBoard(state) {
  const width = Math.max(72, process.stdout.columns || 100)
  const height = Math.max(20, process.stdout.rows || 30)
  const layout = boardMetrics(state, width, height)
  const { leftWidth, rightWidth, bodyHeight, help, toolbar, tabs } = layout
  const module = state.module
  const filtered = state.filteredItems
  const current = filtered[state.selected] ?? null
  const moduleLabels = state.modules.map((item) => item === module
    ? paint(`[${item.label}]`, COLORS.accent, COLORS.bold)
    : paint(item.label, COLORS.muted))
  const stateColor = state.stateFilter === 'open' ? COLORS.green : COLORS.yellow
  const header = [
    fit(`${paint('◆', COLORS.accent)} ${paint('GitHub Ops', COLORS.bright, COLORS.bold)}  ${paint(state.repo.name, COLORS.cyan)}`, width),
    fit(`  ${moduleLabels.join('   ')}    ${paint('STATE', COLORS.muted)} ${paint(state.stateFilter.toUpperCase(), stateColor, COLORS.bold)}  ${paint('FILTER', COLORS.muted)} ${paint(state.filter || '—', COLORS.bright)}  ${paint('O repo · R refresh', COLORS.muted)}`, width),
    `${COLORS.border}${'─'.repeat(width)}${RESET}`,
  ]
  const listBody = [listHeader(module.kind)]
  if (!filtered.length) listBody.push(state.loading ? 'Loading…' : 'No matching items.')
  for (const [index, item] of filtered.entries()) {
    const row = listRow(module.kind, item, leftWidth)
    listBody.push(index === state.selected
      ? `${paint('▌', COLORS.accent)} ${paint(row, COLORS.bright, COLORS.bold)}`
      : `  ${paint(row, COLORS.muted)}`)
  }
  const tabLines = tabs.lines.map((line) => line.replace(/\[([^\]]+)\]/g, (_match, label) => paint(`[${label}]`, COLORS.accent, COLORS.bold)))
  const details = detailLines(module, current, state.detail, state.detailTab, Math.max(8, rightWidth - 5))
    .map((line) => line.replace(/^([^:]+:)/, (_match, label) => paint(label, COLORS.cyan, COLORS.bold)))
  const rightBody = [...tabLines, '', ...details]
  const toolbarLines = toolbar.lines.map((line) => line.replace(/\[([^\]]+)\]/g, (_match, label) => paint(`[${label}]`, COLORS.accent, COLORS.bold)))
  const helpLines = help.map((line) => paint(line, COLORS.muted))
  const status = state.status || 'Issues, pull requests, tags, releases, and Actions runs are loaded from the focused Herdr pane.'
  const statusStyle = /failed|error|cancelled/i.test(status) ? COLORS.red : COLORS.cyan
  const lines = [
    ...header,
    ...joinPanels(
      panel(`${module.label} (${filtered.length})`, listBody, leftWidth, bodyHeight),
      panel(current ? `Selected #${current.number}` : 'Details', rightBody, rightWidth, bodyHeight),
      leftWidth,
      rightWidth,
    ),
    ...helpLines.map((line) => fit(line, width)),
    ...toolbarLines.map((line) => fit(line, width)),
    fit(paint(status, statusStyle), width),
  ]

  if (state.modal) return drawModal(lines, state.modal, width, height)
  return lines.slice(0, height).join('\n')
}

function drawModal(base, modal, width, height) {
  const contentWidth = Math.min(width - 8, 88)
  const inner = contentWidth - 4
  const body = modal.lines.slice(0, Math.max(3, height - 8))
  const title = ` ${modal.title} `
  const modalTitle = `${COLORS.border}╭${RESET}${paint(title, COLORS.accent, COLORS.bold)}${COLORS.border}${'─'.repeat(Math.max(0, contentWidth - title.length - 2))}╮${RESET}`
  const box = [
    modalTitle,
    ...body.map((line) => {
      const styled = line.replace(/\[([^\]]+)\]/g, (_match, label) => {
        const color = label === 'Cancel' ? COLORS.muted : /default/i.test(label) ? COLORS.yellow : COLORS.cyan
        return paint(`[${label}]`, color, COLORS.bold)
      })
      return `${COLORS.border}│${RESET} ${fit(styled, inner)} ${COLORS.border}│${RESET}`
    }),
    `${COLORS.border}╰${'─'.repeat(Math.max(0, contentWidth - 2))}╯${RESET}`,
  ]
  const top = Math.max(3, Math.floor((height - box.length) / 2))
  const left = Math.max(2, Math.floor((width - contentWidth) / 2))
  const result = [...base]
  while (result.length < height) result.push('')
  for (let index = 0; index < box.length && top + index < height; index++) {
    result[top + index] = `${' '.repeat(left)}${box[index]}`
  }
  return result.slice(0, height).join('\n')
}
