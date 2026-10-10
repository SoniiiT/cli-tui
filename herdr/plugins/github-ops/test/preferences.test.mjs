import assert from 'node:assert/strict'
import { mkdtempSync, readFileSync, rmSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import test from 'node:test'
import { preferencesPath, readPreferences, saveDefaultAgentKind } from '../lib/preferences.mjs'

test('stores the selected default in the Herdr plugin config directory', () => {
  const home = mkdtempSync(path.join(os.tmpdir(), 'github-ops-preferences-'))
  const filePath = path.join(home, 'plugins', 'config', 'soniiit.github-ops', 'preferences.json')
  try {
    assert.equal(
      preferencesPath({ home }),
      path.join(home, '.config', 'herdr', 'plugins', 'config', 'soniiit.github-ops', 'preferences.json'),
    )
    assert.deepEqual(saveDefaultAgentKind('opencode', { filePath }), { defaultAgentKind: 'opencode' })
    assert.equal(readPreferences({ filePath }).defaultAgentKind, 'opencode')
    assert.equal(JSON.parse(readFileSync(filePath, 'utf8')).defaultAgentKind, 'opencode')
  } finally {
    rmSync(home, { recursive: true, force: true })
  }
})

test('preserves existing plugin preferences when changing the default agent', () => {
  const home = mkdtempSync(path.join(os.tmpdir(), 'github-ops-preferences-'))
  const filePath = path.join(home, 'preferences.json')
  try {
    saveDefaultAgentKind('codex', { filePath })
    saveDefaultAgentKind('opencode', { filePath })
    assert.deepEqual(readPreferences({ filePath }), { defaultAgentKind: 'opencode' })
  } finally {
    rmSync(home, { recursive: true, force: true })
  }
})
