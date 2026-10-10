import assert from 'node:assert/strict'
import test from 'node:test'
import { displayWidth, hitTestBoard, renderBoard, sanitizeTerminalText, toolbarItems, wrap } from '../lib/terminal.mjs'

const modules = [
  { label: 'Issues', kind: 'issue', detailTabs: ['Overview', 'Description', 'Comments', 'Related'] },
  { label: 'Pull Requests', kind: 'pull', detailTabs: ['Overview', 'Description', 'Comments', 'Reviews', 'Files', 'Checks'] },
  { label: 'Tags', kind: 'tag', detailTabs: ['Overview', 'Commit'] },
  { label: 'Releases', kind: 'release', detailTabs: ['Overview', 'Notes', 'Assets'] },
  { label: 'Actions', kind: 'run', detailTabs: ['Overview', 'Jobs'] },
]

function boardState(moduleIndex = 0, modal = null) {
  return {
    repo: { name: 'acme/shop' },
    modules,
    module: modules[moduleIndex],
    moduleIndex,
    detailTab: 'Overview',
    stateFilter: 'open',
    filter: '',
    filteredItems: [{ number: 42 }, { number: 43 }],
    selected: 0,
    modal,
  }
}

function findTarget(state, predicate, width = 120, height = 40) {
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const target = hitTestBoard(state, x, y, width, height)
      if (target && predicate(target)) return { x, y, target }
    }
  }
  assert.fail('Expected click target was not laid out')
}

test('maps module labels, visible issue rows, and detail tabs to click targets', () => {
  const state = boardState()
  assert.equal(hitTestBoard(state, 2, 1, 120, 40).index, 0)
  assert.equal(findTarget(state, (target) => target.type === 'module' && target.index === 1).target.index, 1)
  assert.deepEqual(hitTestBoard(state, 4, 5, 120, 40), { type: 'item', index: 0 })
  assert.equal(findTarget(state, (target) => target.type === 'detail-tab' && target.tab === 'Description').target.tab, 'Description')
})

test('renders a colored, rounded terminal board within its column width', () => {
  const output = renderBoard(boardState())
  const plain = output.replace(/\u001b\[[0-?]*[ -/]*[@-~]/g, '')
  const width = Math.max(72, process.stdout.columns || 100)

  assert.match(output, /\u001b\[38;2;/)
  assert.match(plain, /◆ GitHub Ops/)
  assert.match(plain, /╭ Issues/)
  assert.ok(plain.split('\n').every((line) => displayWidth(line) <= width))
})

test('strips control sequences from GitHub text while preserving board theme colors', () => {
  assert.equal(
    sanitizeTerminalText('Description\u001b[2Jhidden\ttext\u001b[38;2;121;192;255mblue\u001b[0m'),
    'Descriptionhidden    text\u001b[38;2;121;192;255mblue\u001b[0m',
  )
})

test('keeps the board header visible when a PR description contains terminal controls', () => {
  const state = boardState(1)
  state.detailTab = 'Description'
  state.filteredItems[0] = { number: 49, title: 'Test PR' }
  state.detail = { number: 49, title: 'Test PR', body: 'Intro\u001b[2Jhidden\nMore details' }

  const output = renderBoard(state)
  const plain = output.replace(/\u001b\[[0-?]*[ -/]*[@-~]/g, '')

  assert.match(plain, /GitHub Ops/)
  assert.match(plain, /Test PR/)
  assert.equal(output.includes('\u001b[2J'), false)
})

test('wraps wide emoji and CJK graphemes by terminal columns', () => {
  const lines = wrap('📅 schedule 👨‍👩‍👧‍👦 東京 '.repeat(5), 18)

  assert.ok(lines.length > 1)
  assert.ok(lines.every((line) => displayWidth(line) <= 18))
})

test('lays out clickable action buttons and delegation modal controls', () => {
  const state = boardState(1)
  assert.equal(findTarget(state, (target) => target.type === 'action' && target.action === 'm').target.action, 'm')

  state.modal = {
    type: 'delegate',
    candidates: [{ label: 'agent one' }, { label: 'agent two' }],
    lines: ['Delegate PR #42', 'Choose an agent', '▶ agent one', '  agent two', '[Continue]   [Cancel]'],
  }
  assert.equal(findTarget(state, (target) => target.type === 'modal-choice' && target.index === 1).target.index, 1)
  assert.equal(findTarget(state, (target) => target.type === 'modal-primary').target.type, 'modal-primary')
  assert.equal(findTarget(state, (target) => target.type === 'modal-cancel').target.type, 'modal-cancel')
})

test('tag/release/run modules switch and render without a Close/Reopen button', () => {
  const tagState = boardState(2)
  tagState.filteredItems = [{ number: 'v1.2.3', title: 'v1.2.3', sha: 'abc1234' }]
  assert.equal(findTarget(tagState, (target) => target.type === 'module' && target.index === 2).target.index, 2)
  assert.equal(toolbarItems(modules[2]).some((item) => item.action === 'x'), false)
  assert.doesNotThrow(() => renderBoard(tagState))

  const releaseState = boardState(3)
  releaseState.filteredItems = [{ number: 'v1.2.3', title: 'v1.2.3', isDraft: true }]
  assert.equal(toolbarItems(modules[3]).some((item) => item.action === 'release-delete'), true)
  assert.doesNotThrow(() => renderBoard(releaseState))

  const runState = boardState(4)
  runState.filteredItems = [{ number: 42, title: 'CI', status: 'completed', conclusion: 'failure' }]
  const runToolbar = toolbarItems(modules[4])
  assert.equal(runToolbar.some((item) => item.action === 'run-cancel'), true)
  assert.equal(runToolbar.some((item) => item.action === 'x' && item.label === 'Close'), false)
  assert.doesNotThrow(() => renderBoard(runState))
})

test('supports mouse-selectable PR-mode and agent-kind menus', () => {
  const state = boardState(1, {
    type: 'menu',
    options: [{ value: 'review', label: 'Review only' }, { value: 'fix', label: 'Local follow-up from PR head' }],
    selected: 0,
    choiceStart: 1,
    buttonLineIndex: 3,
    primaryButton: 'Continue',
    lines: ['Choose PR task mode', '▶ Review only', '  Local follow-up from PR head', '[Continue]   [Cancel]'],
  })
  assert.equal(findTarget(state, (target) => target.type === 'modal-choice' && target.index === 1).target.index, 1)
  assert.equal(findTarget(state, (target) => target.type === 'modal-primary').target.type, 'modal-primary')
})
