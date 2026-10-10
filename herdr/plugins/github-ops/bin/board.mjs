#!/usr/bin/env node
import { emitKeypressEvents } from 'node:readline'
import { createInterface } from 'node:readline/promises'
import { stdin, stdout } from 'node:process'
import path from 'node:path'
import { pluginContext } from '../lib/context.mjs'
import { MODULES } from '../lib/modules.mjs'
import { hitTestBoard, renderBoard, toolbarItems, wrap } from '../lib/terminal.mjs'
import { SgrMouseParser } from '../lib/mouse.mjs'
import {
  closeOrReopen,
  commentTarget,
  editTarget,
  listPullRequests,
  mergePullRequest,
  repository,
  reviewPullRequest,
} from '../lib/github.mjs'
import { createTagRef, resolveRef } from '../lib/tags.mjs'
import { createRelease, deleteRelease, publishRelease } from '../lib/releases.mjs'
import { cancelRun, listWorkflows, rerunRun, runLogs, triggerWorkflow } from '../lib/workflow-runs.mjs'
import { createWorktreeForAgent } from '../lib/delegation.mjs'
import { delegatedPrompt, parseGithubTarget } from '../lib/target.mjs'
import { runInteractive } from '../lib/process.mjs'
import { readPreferences, saveDefaultAgentKind } from '../lib/preferences.mjs'
import {
  focusAgent,
  getAgent,
  listAgents,
  listAgentKinds,
  openAgentPopup,
  promptAgent,
  startAgent,
  typePrompt,
} from '../lib/herdr.mjs'

const pluginId = 'soniiit.github-ops'
const context = pluginContext()
const cwd = context.cwd
const mode = context.mode
const detailCache = new Map()
const state = {
  repo: null,
  moduleIndex: mode === 'approve' ? 1 : 0,
  stateFilter: 'open',
  filter: '',
  items: [],
  filteredItems: [],
  selected: 0,
  detail: null,
  detailTab: 'Overview',
  loading: false,
  busy: false,
  prompting: false,
  status: 'Loading…',
  modal: null,
  pendingDelegation: null,
  initialTarget: null,
}

let keypressHandler = null
let rawMode = false
let keypressEventsReady = false
let mouseDataHandler = null
let mouseParser = null
let mouseKeypressSequence = false

const KIND_LABELS = { pull: 'PR', issue: 'Issue', tag: 'Tag', release: 'Release', run: 'Run' }

function currentModule() {
  return MODULES[state.moduleIndex]
}

function selectedItem() {
  return state.filteredItems[state.selected] ?? null
}

function selectedTarget() {
  const item = selectedItem()
  if (!item) return null
  return currentModule().toTarget ? currentModule().toTarget(item) : parseGithubTarget(item.url)
}

function filterItems() {
  const query = state.filter.trim().toLowerCase()
  state.filteredItems = query
    ? state.items.filter((item) => [
      item.number,
      item.title,
      item.author?.login,
      ...(item.labels ?? []).map((label) => label.name),
      ...(item.assignees ?? []).map((assignee) => assignee.login),
    ].filter(Boolean).join(' ').toLowerCase().includes(query))
    : [...state.items]
  state.selected = Math.min(state.selected, Math.max(0, state.filteredItems.length - 1))
}

function draw() {
  if (!rawMode) return
  stdout.write('\u001b[?25l\u001b[2J\u001b[H')
  stdout.write(renderBoard(renderState()))
}

function startKeyboard() {
  if (!stdin.isTTY || typeof stdin.setRawMode !== 'function') return false
  if (!keypressEventsReady) {
    mouseParser = new SgrMouseParser()
    mouseDataHandler = (chunk) => {
      const { events } = mouseParser.push(chunk)
      for (const event of events) void handleMouseClick(event)
    }
    stdin.on('data', mouseDataHandler)
    emitKeypressEvents(stdin)
    keypressEventsReady = true
  } else if (mouseDataHandler) {
    stdin.on('data', mouseDataHandler)
  }
  if (keypressHandler) stdin.on('keypress', keypressHandler)
  stdin.setRawMode(true)
  stdin.resume()
  rawMode = true
  stdout.write('\u001b[?1000h\u001b[?1006h')
  return true
}

function stopKeyboard() {
  if (keypressHandler) stdin.removeListener('keypress', keypressHandler)
  if (mouseDataHandler) stdin.removeListener('data', mouseDataHandler)
  if (stdin.isTTY && typeof stdin.setRawMode === 'function') stdin.setRawMode(false)
  rawMode = false
  stdout.write('\u001b[?1006l\u001b[?1000l\u001b[?25h')
}

async function showLinePrompt(label) {
  state.prompting = true
  stopKeyboard()
  const line = createInterface({ input: stdin, output: stdout })
  try {
    return await line.question(label)
  } finally {
    line.close()
    state.prompting = false
    startKeyboard()
    draw()
  }
}

async function showBlockPrompt(label) {
  state.prompting = true
  stopKeyboard()
  const line = createInterface({ input: stdin, output: stdout })
  try {
    console.log(`${label} (type one line at a time; a single dot finishes)`)
    const rows = []
    for (;;) {
      const value = await line.question('| ')
      if (value === '.') break
      rows.push(value)
    }
    return rows.join('\n')
  } finally {
    line.close()
    state.prompting = false
    startKeyboard()
    draw()
  }
}

function loadSelectedDetail() {
  const target = selectedTarget()
  if (!target) {
    state.detail = null
    return
  }
  const key = `${target.nameWithOwner}:${target.kind}:${target.number}`
  try {
    if (!detailCache.has(key)) detailCache.set(key, currentModule().detail(target, cwd))
    state.detail = detailCache.get(key)
  } catch (error) {
    state.detail = null
    state.status = `Details unavailable: ${error.message}`
  }
}

function refresh() {
  state.loading = true
  state.status = `Refreshing ${currentModule().label.toLowerCase()}…`
  draw()
  try {
    state.items = currentModule().load(state.repo.name, cwd, state.stateFilter)
    if (state.initialTarget && state.initialTarget.kind === currentModule().kind) {
      const index = state.items.findIndex((item) => item.number === state.initialTarget.number)
      if (index < 0) {
        const detail = currentModule().detail(state.initialTarget, cwd)
        detailCache.set(`${state.initialTarget.nameWithOwner}:${state.initialTarget.kind}:${state.initialTarget.number}`, detail)
        state.items.unshift({ ...detail, kind: state.initialTarget.kind })
      }
    }
    filterItems()
    if (state.initialTarget && currentModule().kind === state.initialTarget.kind) {
      const index = state.filteredItems.findIndex((item) => item.number === state.initialTarget.number)
      if (index >= 0) state.selected = index
      state.initialTarget = null
    }
    loadSelectedDetail()
    state.status = `${state.filteredItems.length} ${currentModule().label.toLowerCase()} · ${state.repo.name}`
  } catch (error) {
    state.items = []
    state.filteredItems = []
    state.detail = null
    state.status = `Refresh failed: ${error.message}`
  } finally {
    state.loading = false
    draw()
  }
}

function moveSelection(delta) {
  if (!state.filteredItems.length) return
  state.selected = Math.min(state.filteredItems.length - 1, Math.max(0, state.selected + delta))
  loadSelectedDetail()
  draw()
}

function changeModule(index) {
  state.moduleIndex = (index + MODULES.length) % MODULES.length
  state.selected = 0
  state.detailTab = currentModule().detailTabs[0]
  refresh()
}

function changeDetailTab(delta) {
  const tabs = currentModule().detailTabs
  const selected = Math.max(0, tabs.indexOf(state.detailTab))
  state.detailTab = tabs[(selected + delta + tabs.length) % tabs.length]
  draw()
}

async function changeRepository() {
  const value = (await showLinePrompt(`Repository owner/name [${state.repo.name}]: `)).trim()
  if (!value) return
  try {
    state.repo = repository(cwd, value)
    state.filter = ''
    state.selected = 0
    state.initialTarget = null
    detailCache.clear()
    refresh()
  } catch (error) {
    state.status = `Repository unavailable: ${error.message}`
    draw()
  }
}

function eligibleAgents() {
  const agents = listAgents()
    .filter((agent) => agent.pane_id && agent.pane_id !== context.paneId && ['idle', 'done'].includes(agent.agent_status))
    .map((agent) => ({
      type: 'agent',
      paneId: agent.pane_id,
      kind: agent.agent || agent.kind || 'agent',
      status: agent.agent_status,
      cwd: agent.cwd || '',
      label: `${agent.display_agent || agent.agent || agent.kind || 'Agent'} · ${agent.pane_id} · ${agent.agent_status} · ${agent.cwd || ''}`,
    }))
  agents.sort((left, right) => {
    const a = left.paneId === context.sourcePaneId ? 0 : 1
    const b = right.paneId === context.sourcePaneId ? 0 : 1
    return a - b || left.label.localeCompare(right.label)
  })
  return agents
}

function openDelegateModal(target = selectedTarget(), item = state.detail ?? selectedItem()) {
  if (!target || !item) {
    state.status = 'Select an item first.'
    draw()
    return
  }
  if (target.kind === 'run' && target.failedLog === undefined) {
    try {
      target = { ...target, failedLog: runLogs(target, cwd, { failedOnly: true }) || '(no failed step output)' }
    } catch (error) {
      target = { ...target, failedLog: `(failed to fetch logs: ${error.message})` }
    }
  }
  let candidates = []
  try {
    candidates = eligibleAgents()
  } catch (error) {
    state.status = `Could not list Herdr agents: ${error.message}`
  }
  candidates.push({ type: 'worktree', label: '＋ Create new Agent popup' })
  const preferred = candidates.findIndex((candidate) => candidate.paneId === context.sourcePaneId)
  state.modal = {
    type: 'delegate',
    title: `Delegate ${KIND_LABELS[target.kind] ?? target.kind} #${target.number}`,
    target,
    item,
    candidates,
    selected: Math.max(0, preferred),
  }
  draw()
}

function modalPresentation(modal) {
  if (modal.type === 'delegate') {
    return {
      ...modal,
      choiceStart: 2,
      buttonLineIndex: 2 + modal.candidates.length,
      primaryButton: 'Continue',
      lines: [
        `Delegate ${KIND_LABELS[modal.target.kind] ?? modal.target.kind} #${modal.target.number}`,
        `Choose an idle agent or create a new popup in ${state.repo.name}:`,
        ...modal.candidates.map((candidate, index) => `${index === modal.selected ? '▶ ' : '  '}${candidate.label}`),
        '[Continue]   [Cancel]',
      ],
    }
  }
  if (modal.type === 'dropdown') {
    const pageSize = 7
    const maxOffset = Math.max(0, modal.options.length - pageSize)
    let offset = Math.max(0, Math.min(modal.offset ?? 0, maxOffset))
    if (modal.selected < offset) offset = modal.selected
    else if (modal.selected >= offset + pageSize) offset = modal.selected - pageSize + 1
    const selectedOption = modal.options[modal.selected]
    const showDefault = modal.showDefaultButton !== false
    const defaultLine = showDefault ? `Saved default: ${modal.savedDefault ?? readPreferences().defaultAgentKind ?? 'not set'}` : ''
    const defaultButtonText = showDefault ? '[D Save default]   ' : ''
    if (!modal.expanded) {
      return {
        ...modal,
        choiceStart: 4,
        buttonLineIndex: 3,
        dropdownToggleLine: 1,
        primaryButton: modal.primaryButton || 'Continue',
        defaultButton: showDefault ? 'D Save default' : undefined,
        lines: [
          modal.description || 'Click the selector or press ↓ to open choices.',
          `[${selectedOption?.label ?? 'select'} ▾]`,
          defaultLine,
          `${defaultButtonText}[${modal.primaryButton || 'Continue'}]   [Cancel]`,
        ],
      }
    }
    const visibleOptions = modal.options.slice(offset, offset + pageSize)
    return {
      ...modal,
      choiceStart: 2,
      choiceOffset: offset,
      visibleChoices: visibleOptions,
      dropdownToggleLine: 1,
      buttonLineIndex: 3 + visibleOptions.length,
      primaryButton: modal.primaryButton || 'Continue',
      defaultButton: showDefault ? 'D Save default' : undefined,
      lines: [
        `${modal.description || 'Choose an option;'} ↑↓ navigates, click selects.`,
        `[${selectedOption?.label ?? 'select'} ▴]`,
        ...visibleOptions.map((option, index) => `${offset + index === modal.selected ? '▶ ' : '  '}${option.label}`),
        `Showing ${offset + 1}–${offset + visibleOptions.length} of ${modal.options.length}`,
        `${defaultButtonText}[${modal.primaryButton || 'Continue'}]   [Cancel]`,
      ],
    }
  }
  if (modal.type === 'menu') {
    return {
      ...modal,
      choiceStart: 1,
      buttonLineIndex: 1 + modal.options.length,
      primaryButton: modal.primaryButton || 'Continue',
      lines: [
        modal.description,
        ...modal.options.map((option, index) => `${index === modal.selected ? '▶ ' : '  '}${option.label}`),
        `[${modal.primaryButton || 'Continue'}]   [Cancel]`,
      ],
    }
  }
  if (modal.type === 'review') {
    const button = modal.primaryButton || 'Start OpenCode'
    const cancelButton = modal.hideCancel ? '' : '   [Cancel]'
    return {
      ...modal,
      choiceStart: modal.reviewLines.length,
      buttonLineIndex: modal.reviewLines.length,
      primaryButton: button,
      lines: [...modal.reviewLines, `[${button}]${cancelButton}`],
    }
  }
  if (modal.type === 'form') {
    const lineFieldMap = []
    const fieldLines = modal.fields.flatMap((field, index) => {
      const focused = index === modal.focused
      const marker = focused ? '▶ ' : '  '
      let rendered
      if (field.type === 'select') {
        const option = field.options.find((candidate) => candidate.value === field.value) ?? field.options[0]
        rendered = [`${marker}${field.label}: ◀ ${option?.label ?? ''} ▶`]
      } else if (field.type === 'textarea') {
        const bodyLines = field.value.split('\n')
        rendered = bodyLines.map((line, lineIndex) => lineIndex === 0
          ? `${marker}${field.label}: ${line}${focused && bodyLines.length === 1 ? '▏' : ''}`
          : `    ${line}${focused && lineIndex === bodyLines.length - 1 ? '▏' : ''}`)
      } else {
        rendered = [`${marker}${field.label}: ${field.value}${focused ? '▏' : ''}`]
      }
      for (const _line of rendered) lineFieldMap.push(index)
      return rendered
    })
    return {
      ...modal,
      fieldStart: 1,
      lineFieldMap,
      buttonLineIndex: 2 + fieldLines.length,
      primaryButton: modal.primaryButton || 'Continue',
      lines: [
        modal.description || '',
        ...fieldLines,
        '',
        `[${modal.primaryButton || 'Continue'}]   [Cancel]`,
      ],
    }
  }
  return modal
}

function displayError(error) {
  const message = error?.message ?? String(error)
  try {
    const parsed = JSON.parse(message)
    return parsed?.error?.message ?? parsed?.message ?? message
  } catch {
    return message
  }
}

async function sendToExistingAgent(candidate, modal) {
  try {
    const current = getAgent(candidate.paneId)
    const agent = current?.agent ?? current ?? {}
    if (!['idle', 'done'].includes(agent.agent_status)) {
      state.status = `${candidate.kind} is no longer idle; choose another agent.`
      state.modal = null
      draw()
      return
    }
    promptAgent(candidate.paneId, delegatedPrompt(modal.target, modal.item.title, modal.target.kind === 'pull' ? 'review' : 'work'))
    state.status = `Sent ${modal.target.url} to ${candidate.kind}.`
  } catch (error) {
    state.status = `Agent delegation failed: ${displayError(error)}`
  }
  state.modal = null
  state.pendingDelegation = null
  draw()
}

function openMenu(step, title, description, options, selected = 0) {
  state.modal = {
    type: 'menu',
    step,
    title,
    description,
    options,
    selected: Math.max(0, selected),
    primaryButton: 'Continue',
  }
  draw()
}

function openAgentKindMenu() {
  const kinds = listAgentKinds()
  const storedDefault = readPreferences().defaultAgentKind
  const preferred = kinds.includes(storedDefault)
    ? storedDefault
    : kinds.includes(context.focusedAgent) ? context.focusedAgent : kinds.includes('opencode') ? 'opencode' : kinds[0]
  state.modal = {
    type: 'dropdown',
    step: 'agent-kind',
    title: 'Choose agent type',
    description: 'Select which coding agent to open in the zoomed popup:',
    options: kinds.map((kind) => ({ value: kind, label: kind })),
    selected: Math.max(0, kinds.indexOf(preferred)),
    offset: 0,
    expanded: false,
    savedDefault: storedDefault || '',
    primaryButton: 'Continue',
  }
  draw()
}

function saveSelectedAgentDefault(modal = state.modal) {
  const option = modal?.options?.[modal.selected]
  if (!option) return
  saveDefaultAgentKind(option.value)
  if (state.modal) state.modal.savedDefault = option.value
  state.status = `${option.value} is now the default GitHub Ops agent.`
  draw()
}

function beginNewAgent(modal) {
  state.pendingDelegation = {
    target: modal.target,
    item: modal.item,
    repoInfo: state.repo,
    cwd,
    mode: modal.target.kind === 'pull' ? null : 'work',
    kind: null,
    extra: '',
  }
  if (modal.target.kind === 'pull') {
    openMenu('pr-mode', 'Choose PR task', 'What should the new agent do?', [
      { value: 'review', label: 'Review only · read-only' },
      { value: 'fix', label: 'Local follow-up · start from the PR head' },
    ])
  } else {
    openAgentKindMenu()
  }
}

function openExtraInstructionMenu() {
  openMenu('extra', 'Optional instruction', 'Add a note to the task prompt or continue with the standard prompt:', [
    { value: 'none', label: 'Use standard task prompt' },
    { value: 'custom', label: 'Add an extra instruction…' },
  ])
}

function openStartReview() {
  const pending = state.pendingDelegation
  const prompt = [delegatedPrompt(pending.target, pending.item.title, pending.mode), pending.extra]
    .filter(Boolean)
    .join(' ')
    .replace(/\s+/g, ' ')
    .trim()
  pending.prompt = prompt
  const targetLabel = `${KIND_LABELS[pending.target.kind] ?? pending.target.kind} #${pending.target.number}`
  const modeLabel = pending.mode === 'fix' ? 'Local follow-up from PR head' : pending.mode === 'review' ? 'Read-only PR review' : 'Issue implementation'
  const previewWidth = Math.max(30, Math.min(78, (stdout.columns || 100) - 24))
  state.modal = {
    type: 'review',
    title: 'Review new Agent popup',
    reviewLines: [
      `${state.repo.name} · ${targetLabel}`,
      `Agent: ${pending.kind} · Mode: ${modeLabel}`,
      'Opens in this Herdr workspace; the Git worktree is isolated.',
      'The task will be typed into the popup, not submitted.',
      'Prompt preview:',
      ...wrap(prompt, previewWidth).slice(0, 5),
    ],
    primaryButton: 'Start OpenCode',
  }
  draw()
}

function cancelDelegation(message = 'Cancelled; no changes were made.') {
  state.modal = null
  state.pendingDelegation = null
  state.status = message
  draw()
}

async function launchNewWorktreeAgent() {
  const pending = state.pendingDelegation
  if (!pending) return
  const { kind, prompt } = pending
  state.modal = null
  state.busy = true
  state.status = `Creating an isolated worktree and zoomed popup for ${pending.target.url}…`
  draw()
  let worktreePath = ''
  let stage = 'worktree'
  try {
    const result = createWorktreeForAgent({
      target: pending.target,
      title: pending.item.title,
      repoInfo: pending.repoInfo,
      cwd: pending.cwd,
      mode: pending.mode,
    })
    worktreePath = result?.worktree?.path || ''
    if (!worktreePath) throw new Error('Git created the worktree but returned no checkout path.')

    stage = 'popup'
    const popup = openAgentPopup({ pluginId, workspaceId: context.workspaceId, cwd: worktreePath })
    const paneId = popup?.pane_id
    if (!paneId) throw new Error('Herdr opened the worktree popup but returned no pane ID.')

    const name = `github-${pending.target.kind}-${pending.target.number}-${Date.now().toString(36)}`
    stage = 'agent'
    startAgent({ paneId, name, kind })
    focusAgent(paneId)

    stage = 'prompt'
    typePrompt(paneId, prompt)
    state.status = `Started ${kind} in a focused zoomed popup in this workspace at ${worktreePath}; review the task prompt and press Enter to send it.`
  } catch (error) {
    const detail = displayError(error)
    state.status = stage === 'worktree'
      ? `Worktree creation failed: ${detail}`
      : `Worktree created at ${worktreePath}, but ${stage} startup failed: ${detail}`
  } finally {
    state.pendingDelegation = null
    state.busy = false
    draw()
  }
}

async function handleModalPrimary() {
  const modal = state.modal
  if (modal.type === 'form') return submitForm()
  if (modal.type === 'delegate') {
    const candidate = modal.candidates[modal.selected]
    if (candidate?.type === 'agent') {
      state.modal = null
      state.pendingDelegation = null
      await sendToExistingAgent(candidate, modal)
    } else if (candidate?.type === 'worktree') {
      beginNewAgent(modal)
    }
    return
  }
  if (modal.type === 'dropdown') {
    const option = modal.options[modal.selected]
    if (!option) return
    if (modal.onSelect) return modal.onSelect(option)
    state.pendingDelegation.kind = option.value
    openExtraInstructionMenu()
    return
  }
  if (modal.type === 'menu') {
    const option = modal.options[modal.selected]
    if (!option) return
    if (modal.onSelect) return modal.onSelect(option)
    if (modal.step === 'pr-mode') {
      state.pendingDelegation.mode = option.value
      openAgentKindMenu()
    } else if (modal.step === 'agent-kind') {
      state.pendingDelegation.kind = option.value
      openExtraInstructionMenu()
    } else if (modal.step === 'extra') {
      if (option.value === 'custom') {
        state.modal = null
        state.pendingDelegation.extra = (await showLinePrompt('Extra instruction (optional): ')).trim()
      }
      openStartReview()
    }
    return
  }
  if (modal.type === 'review') {
    if (modal.onConfirm) return modal.onConfirm()
    await launchNewWorktreeAgent()
  }
}

async function submitForm() {
  const modal = state.modal
  const values = Object.fromEntries(modal.fields.map((field) => [field.id, field.value]))
  state.modal = null
  if (modal.onSubmit) await modal.onSubmit(values)
  draw()
}

async function handleFormKey(key, text) {
  const modal = state.modal
  const field = modal.fields[modal.focused]
  const isLast = modal.focused === modal.fields.length - 1
  if (key.name === 'escape') return cancelDelegation()
  if (key.name === 'tab' || (key.name === 'down' && field.type !== 'textarea')) {
    modal.focused = (modal.focused + 1) % modal.fields.length
    draw()
    return
  }
  if (key.name === 'up' && field.type !== 'textarea') {
    modal.focused = (modal.focused + modal.fields.length - 1) % modal.fields.length
    draw()
    return
  }
  if (field.type === 'select' && (key.name === 'left' || key.name === 'right')) {
    const options = field.options
    const currentIndex = Math.max(0, options.findIndex((option) => option.value === field.value))
    const delta = key.name === 'right' ? 1 : -1
    field.value = options[(currentIndex + delta + options.length) % options.length].value
    draw()
    return
  }
  if (field.type === 'textarea') {
    // Enter always inserts a newline here (release notes etc. need real line breaks);
    // use Tab/↑↓ to leave the field instead of Enter, which would just add blank lines.
    if (key.name === 'return') {
      field.value += '\n'
      draw()
      return
    }
    if (key.name === 'backspace') {
      field.value = field.value.slice(0, -1)
      draw()
      return
    }
    if (text && text.length === 1 && !key.ctrl && !key.meta) {
      field.value += text
      draw()
    }
    return
  }
  if (field.type === 'text' && key.name === 'backspace') {
    field.value = field.value.slice(0, -1)
    draw()
    return
  }
  if (key.name === 'return') {
    if (!isLast) {
      modal.focused += 1
      draw()
      return
    }
    return submitForm()
  }
  if (field.type === 'text' && text && text.length === 1 && !key.ctrl && !key.meta) {
    field.value += text
    draw()
  }
}

async function handleDelegateKey(key) {
  const modal = state.modal
    if (key.name === 'escape' || key.name === 'q') {
    cancelDelegation()
    return
  }
  if (modal.type === 'dropdown' && modal.showDefaultButton !== false && key.name === 'd') {
    saveSelectedAgentDefault(modal)
    return
  }
  const choices = modal.options ?? modal.candidates ?? []
  if (modal.type === 'dropdown' && !modal.expanded && (key.name === 'up' || key.name === 'down')) {
    modal.expanded = true
    draw()
    return
  }
  if ((key.name === 'up' || key.name === 'k') && choices.length) {
    modal.selected = (modal.selected + choices.length - 1) % choices.length
    draw()
    return
  }
  if ((key.name === 'down' || key.name === 'j') && choices.length) {
    modal.selected = (modal.selected + 1) % choices.length
    draw()
    return
  }
  if (key.name === 'return') await handleModalPrimary()
}

function renderState() {
  return {
    ...state,
    module: currentModule(),
    modules: MODULES,
    modal: state.modal ? modalPresentation(state.modal) : null,
  }
}

async function handleMouseClick(point) {
  if (state.busy || state.prompting) return
  const target = hitTestBoard(renderState(), point.x, point.y, stdout.columns, stdout.rows)
  if (!target) return

  state.busy = true
  try {
    if (target.type === 'modal-choice') {
      state.modal.selected = target.index
    } else if (target.type === 'modal-field') {
      state.modal.focused = target.index
    } else if (target.type === 'modal-toggle') {
      state.modal.expanded = !state.modal.expanded
    } else if (target.type === 'modal-default') {
      saveSelectedAgentDefault()
    } else if (target.type === 'modal-primary') {
      await handleModalPrimary()
    } else if (target.type === 'modal-cancel') {
      cancelDelegation()
    } else if (target.type === 'module') {
      changeModule(target.index)
    } else if (target.type === 'item') {
      state.selected = target.index
      loadSelectedDetail()
    } else if (target.type === 'detail-tab') {
      state.detailTab = target.tab
    } else if (target.type === 'action') {
      if (target.action === 'repo') await changeRepository()
      else if (target.action === 'refresh') refresh()
      else if (target.action === 'state') {
        state.stateFilter = state.stateFilter === 'open' ? 'all' : 'open'
        refresh()
      } else if (target.action === 'q') quit()
      else await runBoardAction(target.action)
    }
  } catch (error) {
    state.status = error.message
  } finally {
    state.busy = false
    draw()
  }
}

function ownerRepoOf(nameWithOwner) {
  const [owner, repo] = nameWithOwner.split('/')
  return { owner, repo }
}

// Typed safety confirmation, as a one-field popup form — never a bottom-line readline prompt.
function openConfirmForm({ title, description, phrase, onConfirm }) {
  state.modal = {
    type: 'form',
    step: 'confirm',
    title,
    description,
    fields: [{ id: 'confirm', label: `Type "${phrase}"`, type: 'text', value: '' }],
    focused: 0,
    primaryButton: 'Confirm',
    onSubmit: async (values) => {
      if (values.confirm.trim() !== phrase) {
        state.status = 'Confirmation did not match; no changes were made.'
        return
      }
      await onConfirm()
    },
  }
  draw()
}

// --- Tag creation: one centered form popup (name + target), then a typed safety confirmation. ---

function createNewTag() {
  state.modal = {
    type: 'form',
    step: 'tag-create',
    title: 'New Tag',
    description: `Create a tag in ${state.repo.name}:`,
    fields: [
      { id: 'name', label: 'Name', type: 'text', value: '' },
      { id: 'ref', label: 'Target (branch or SHA)', type: 'text', value: state.repo.defaultBranch },
    ],
    focused: 0,
    primaryButton: 'Create',
    onSubmit: async (values) => {
      const name = values.name.trim()
      if (!name) {
        state.status = 'Tag name is required.'
        return
      }
      finishTagCreation(name, values.ref.trim() || state.repo.defaultBranch)
    },
  }
  draw()
}

function finishTagCreation(name, ref) {
  let sha
  try {
    sha = resolveRef(state.repo.name, ref, cwd)
  } catch (error) {
    state.status = `Could not resolve "${ref}": ${error.message}`
    draw()
    return
  }
  openConfirmForm({
    title: 'Confirm Tag',
    description: `Create tag "${name}" at ${sha.slice(0, 7)}?`,
    phrase: `CREATE ${name}`,
    onConfirm: async () => {
      createTagRef(state.repo.name, name, sha, cwd)
      state.status = `Tag ${name} created at ${sha.slice(0, 7)}.`
      refresh()
    },
  })
}

// --- Release creation: one centered form popup (tag, title, notes source/body, visibility), then a ---
// --- typed safety confirmation popup. The notes body is always present as a textarea field so the ---
// --- whole flow stays in one HUD element; it's only used when Notes source is "Write manually". ---

function createNewRelease() {
  state.modal = {
    type: 'form',
    step: 'release-create',
    title: 'New Release',
    description: `Create a release in ${state.repo.name}:`,
    fields: [
      { id: 'tag', label: 'Tag', type: 'text', value: '' },
      { id: 'title', label: 'Title', type: 'text', value: '' },
      {
        id: 'notesSource', label: 'Notes', type: 'select', value: 'manual',
        options: [
          { value: 'manual', label: 'Write manually' },
          { value: 'generate', label: 'Generate from merged PRs' },
          { value: 'ai', label: 'Delegate draft to an AI agent' },
        ],
      },
      { id: 'notesBody', label: 'Notes text (if writing manually)', type: 'textarea', value: '' },
      {
        id: 'visibility', label: 'Visibility', type: 'select', value: 'draft',
        options: [
          { value: 'draft', label: 'Draft' },
          { value: 'prerelease', label: 'Pre-release' },
          { value: 'publish', label: 'Publish now' },
        ],
      },
    ],
    focused: 0,
    primaryButton: 'Create',
    onSubmit: async (values) => {
      const tag = values.tag.trim()
      if (!tag) {
        state.status = 'Tag is required.'
        return
      }
      const title = values.title.trim()
      if (values.notesSource === 'ai') {
        const syntheticTarget = { kind: 'release', number: tag, nameWithOwner: state.repo.name, ...ownerRepoOf(state.repo.name) }
        openDelegateModal(syntheticTarget, { title: title || tag })
        state.status = `Delegating release-notes drafting for ${tag}. Once the agent responds, rerun "New Release" and paste the notes manually (GitHub Ops does not read agent panes back).`
        return
      }
      const generateNotes = values.notesSource === 'generate'
      const notes = values.notesSource === 'manual' ? values.notesBody.trim() : ''
      finishReleaseCreation(tag, title, notes, generateNotes, values.visibility)
    },
  }
  draw()
}

function finishReleaseCreation(tag, title, notes, generateNotes, visibility) {
  const draft = visibility === 'draft'
  const prerelease = visibility === 'prerelease'
  openConfirmForm({
    title: 'Confirm Release',
    description: `Create release "${tag}"${draft ? ' as a draft' : prerelease ? ' as a prerelease' : ', published now'}?`,
    phrase: `RELEASE ${tag}`,
    onConfirm: async () => {
      createRelease({ tag, repo: state.repo.name, title: title || undefined, notes: notes || undefined, generateNotes, draft, prerelease }, cwd)
      state.status = `Release ${tag} created${draft ? ' as a draft' : prerelease ? ' as a prerelease' : ''}.`
      refresh()
    },
  })
}

// --- Workflow trigger: one centered form popup (workflow, ref, inputs), then a typed safety confirmation. ---

function triggerNewWorkflowRun() {
  let workflows = []
  try {
    workflows = listWorkflows(state.repo.name, cwd)
  } catch (error) {
    state.status = `Could not list workflows: ${error.message}`
    draw()
    return
  }
  if (!workflows.length) {
    state.status = 'No workflows found in this repository.'
    draw()
    return
  }
  state.modal = {
    type: 'form',
    step: 'run-trigger',
    title: 'Trigger Workflow',
    description: `Trigger a workflow in ${state.repo.name}:`,
    fields: [
      {
        id: 'workflow', label: 'Workflow', type: 'select', value: workflows[0].id,
        options: workflows.map((workflow) => ({ value: workflow.id, label: workflow.name })),
      },
      { id: 'ref', label: 'Ref', type: 'text', value: state.repo.defaultBranch },
      { id: 'inputs', label: 'Inputs (key=value, key2=value2)', type: 'text', value: '' },
    ],
    focused: 0,
    primaryButton: 'Trigger',
    onSubmit: async (values) => {
      const workflow = workflows.find((candidate) => candidate.id === values.workflow)
      const ref = values.ref.trim() || state.repo.defaultBranch
      const fields = values.inputs.trim()
        ? values.inputs.split(',').map((pair) => pair.split('=')).filter(([key]) => key?.trim()).map(([key, ...rest]) => [key.trim(), rest.join('=').trim()])
        : []
      finishWorkflowTrigger(workflow, ref, fields)
    },
  }
  draw()
}

function finishWorkflowTrigger(workflow, ref, fields) {
  openConfirmForm({
    title: 'Confirm Workflow Run',
    description: `Trigger "${workflow.name}" on ${ref}?`,
    phrase: `RUN ${workflow.name}`,
    onConfirm: async () => {
      triggerWorkflow(state.repo.name, workflow.id, cwd, { ref, fields })
      state.status = `Triggered ${workflow.name} on ${ref}.`
      refresh()
    },
  })
}

function viewRunLogs(target) {
  let log = ''
  try {
    log = runLogs(target, cwd, { failedOnly: true }) || '(no failed step output; the run may still be in progress or fully succeeded)'
  } catch (error) {
    log = `Could not fetch logs: ${error.message}`
  }
  state.modal = {
    type: 'review',
    title: `Logs: run #${target.number}`,
    reviewLines: wrap(log, Math.min(96, (process.stdout.columns || 100) - 16)),
    primaryButton: 'Close',
    hideCancel: true,
    onConfirm: () => {
      state.modal = null
      draw()
    },
  }
  draw()
}

async function runBoardAction(action) {
  if (action === 'tag-new') return createNewTag()
  if (action === 'release-new') return createNewRelease()
  if (action === 'run-trigger') return triggerNewWorkflowRun()

  const target = selectedTarget()
  const item = selectedItem()
  if (!target || !item) {
    state.status = 'Select an item first.'
    draw()
    return
  }
  if (action === 'run-logs') return viewRunLogs(target)
  const detail = state.detail ?? currentModule().detail(target, cwd)
  if (action === 'v') {
    const pluginRoot = process.env.HERDR_PLUGIN_ROOT || path.resolve(import.meta.dirname, '..')
    const gzg = process.env.GHZINGA_BIN || path.join(pluginRoot, '.runtime', 'bin', 'gzg')
    const session = `github-ops-${state.repo.name.replaceAll('/', '-')}-${target.kind}-${target.number}`
    stopKeyboard()
    try {
      runInteractive(gzg, ['--new', '--session', session, target.url], { cwd })
    } catch (error) {
      state.status = `GHZinga viewer failed: ${error.message}`
    } finally {
      startKeyboard()
      loadSelectedDetail()
      draw()
    }
    return
  }
  if (action === 'd') return openDelegateModal()
  if (action === 'c') {
    const body = (await showBlockPrompt('Comment')).trim()
    if (!body) return
    if ((await showLinePrompt(`Type POST to publish a comment on ${target.url}: `)).trim() !== 'POST') return
    commentTarget(target, cwd, body)
    state.status = 'Comment posted.'
  } else if (action === 'e') {
    const title = (await showLinePrompt('New title (blank keeps current): ')).trim()
    const replaceBody = (await showLinePrompt('Replace the description? [y/N] ')).trim().toLowerCase() === 'y'
    const body = replaceBody ? await showBlockPrompt('New description') : null
    if (!title && body === null) return
    if ((await showLinePrompt(`Type SAVE to update ${target.url}: `)).trim() !== 'SAVE') return
    editTarget(target, cwd, title || null, body)
    state.status = 'Title/description updated.'
  } else if (action === 'a' && target.kind === 'pull') {
    const body = (await showBlockPrompt('Approval summary (optional)')).trim()
    if ((await showLinePrompt(`Type APPROVE to submit the GitHub review for ${target.url}: `)).trim() !== 'APPROVE') return
    reviewPullRequest(target, cwd, 'approve', body)
    state.status = 'Approval review submitted.'
  } else if (action === 'r' && target.kind === 'pull') {
    const body = (await showBlockPrompt('Request-changes summary')).trim()
    if (!body) return
    if ((await showLinePrompt(`Type REQUEST_CHANGES to review ${target.url}: `)).trim() !== 'REQUEST_CHANGES') return
    reviewPullRequest(target, cwd, 'request-changes', body)
    state.status = 'Request-changes review submitted.'
  } else if (action === 'm' && target.kind === 'pull') {
    const method = (await showLinePrompt('Merge method [s]quash, [m]erge, [r]ebase: ')).trim().toLowerCase()
    const selected = ({ s: 'squash', m: 'merge', r: 'rebase' })[method]
    if (!selected) return
    if ((await showLinePrompt(`Type MERGE ${target.number} to merge with ${selected}: `)).trim() !== `MERGE ${target.number}`) return
    mergePullRequest(target, cwd, selected)
    state.status = 'Merge submitted; auto-merge is disabled.'
  } else if (action === 'x' && (target.kind === 'issue' || target.kind === 'pull')) {
    const reopen = detail.state === 'CLOSED'
    const verb = reopen ? 'REOPEN' : 'CLOSE'
    if ((await showLinePrompt(`Type ${verb} to ${verb.toLowerCase()} ${target.url}: `)).trim() !== verb) return
    closeOrReopen(target, cwd, reopen)
    state.status = `${verb} submitted.`
  } else if (action === 'release-publish' && target.kind === 'release') {
    return openConfirmForm({
      title: 'Confirm Publish',
      description: `Publish release "${target.number}"?`,
      phrase: `PUBLISH ${target.number}`,
      onConfirm: async () => {
        publishRelease(target, cwd)
        state.status = `Release ${target.number} published.`
        detailCache.delete(`${target.nameWithOwner}:${target.kind}:${target.number}`)
        refresh()
      },
    })
  } else if (action === 'release-delete' && target.kind === 'release') {
    return openConfirmForm({
      title: 'Confirm Delete',
      description: `Delete release "${target.number}"? This cannot be undone.`,
      phrase: `DELETE ${target.number}`,
      onConfirm: async () => {
        deleteRelease(target, cwd)
        state.status = `Release ${target.number} deleted.`
        detailCache.delete(`${target.nameWithOwner}:${target.kind}:${target.number}`)
        refresh()
      },
    })
  } else if (action === 'run-rerun' && target.kind === 'run') {
    return openConfirmForm({
      title: 'Confirm Rerun',
      description: `Rerun workflow run #${target.number}?`,
      phrase: `RERUN ${target.number}`,
      onConfirm: async () => {
        rerunRun(target, cwd, { failedOnly: false })
        state.status = `Rerun of run #${target.number} requested.`
        detailCache.delete(`${target.nameWithOwner}:${target.kind}:${target.number}`)
        refresh()
      },
    })
  } else if (action === 'run-rerun-failed' && target.kind === 'run') {
    return openConfirmForm({
      title: 'Confirm Rerun Failed Jobs',
      description: `Rerun only the failed jobs of run #${target.number}?`,
      phrase: `RERUN ${target.number}`,
      onConfirm: async () => {
        rerunRun(target, cwd, { failedOnly: true })
        state.status = `Rerun of failed jobs for run #${target.number} requested.`
        detailCache.delete(`${target.nameWithOwner}:${target.kind}:${target.number}`)
        refresh()
      },
    })
  } else if (action === 'run-cancel' && target.kind === 'run') {
    return openConfirmForm({
      title: 'Confirm Cancel',
      description: `Cancel workflow run #${target.number}?`,
      phrase: `CANCEL ${target.number}`,
      onConfirm: async () => {
        cancelRun(target, cwd)
        state.status = `Cancel requested for run #${target.number}.`
        detailCache.delete(`${target.nameWithOwner}:${target.kind}:${target.number}`)
        refresh()
      },
    })
  }
  detailCache.delete(`${target.nameWithOwner}:${target.kind}:${target.number}`)
  refresh()
}

async function handleKeypress(text, key) {
  if (state.busy || state.prompting) return
  state.busy = true
  try {
    if (state.modal?.type === 'form') return await handleFormKey(key, text)
    if (state.modal) return await handleDelegateKey(key)
    if (key.ctrl && key.name === 'c') return quit()
    if (key.name === 'escape' || text === 'q') return quit()
    if (key.name === 'up' || text === 'k') return moveSelection(-1)
    if (key.name === 'down' || text === 'j') return moveSelection(1)
    if (key.name === 'tab') return changeModule(state.moduleIndex + 1)
    const digit = Number(text)
    if (Number.isInteger(digit) && digit >= 1 && digit <= MODULES.length) return changeModule(digit - 1)
    if (text === '[') return changeDetailTab(-1)
    if (text === ']') return changeDetailTab(1)
    if (key.name === 'return') {
      if (!state.detail) return loadSelectedDetail()
      return changeDetailTab(1)
    }
    if (text === '/') {
      state.filter = (await showLinePrompt(`Filter ${currentModule().label.toLowerCase()}: `)).trim()
      filterItems()
      return loadSelectedDetail()
    }
    if (text === 'R' || key.name === 'f5') return refresh()
    if (text === 'O') return changeRepository()
    if (text === 'C') {
      state.stateFilter = state.stateFilter === 'open' ? 'all' : 'open'
      return refresh()
    }
    const toolbarHit = toolbarItems(currentModule()).find((item) => item.key === text
      && !['repo', 'refresh', 'state', 'q'].includes(item.action))
    if (toolbarHit) return await runBoardAction(toolbarHit.action)
  } catch (error) {
    state.status = error.message
  } finally {
    state.busy = false
    draw()
  }
}

function quit() {
  stopKeyboard()
  stdout.write('\u001b[2J\u001b[H')
  process.exit(0)
}

async function main() {
  if (!stdin.isTTY || !stdout.isTTY || typeof stdin.setRawMode !== 'function') {
    console.log('Open GitHub Ops from a Herdr pane: herdr plugin action invoke soniiit.github-ops.open')
    return
  }
  state.status = 'Connecting to GitHub…'
  try {
    state.repo = context.url
      ? repository(cwd, parseGithubTarget(context.url)?.nameWithOwner)
      : repository(cwd)
  } catch (error) {
    console.error(`GitHub Ops: ${error.message}`)
    return
  }
  const target = parseGithubTarget(context.url)
  if (target) {
    state.moduleIndex = Math.max(0, MODULES.findIndex((module) => module.kind === target.kind))
    state.initialTarget = target
    state.stateFilter = 'all'
  } else if (mode === 'approve') {
    state.moduleIndex = Math.max(0, MODULES.findIndex((module) => module.kind === 'pull'))
  } else if (mode === 'create-tag') {
    state.moduleIndex = Math.max(0, MODULES.findIndex((module) => module.kind === 'tag'))
  } else if (mode === 'create-release') {
    state.moduleIndex = Math.max(0, MODULES.findIndex((module) => module.kind === 'release'))
  } else if (mode === 'trigger-workflow') {
    state.moduleIndex = Math.max(0, MODULES.findIndex((module) => module.kind === 'run'))
  }
  keypressHandler = (_text, key) => {
    const sequence = key.sequence ?? ''
    if (mouseKeypressSequence) {
      if (sequence === 'M' || sequence === 'm') mouseKeypressSequence = false
      return
    }
    if (sequence === '\u001b[<') {
      mouseKeypressSequence = true
      return
    }
    if (sequence.startsWith('\u001b[<')) return
    void handleKeypress(_text, key)
  }
  stdout.on('resize', draw)
  refresh()
  startKeyboard()
  draw()
}

main()
