import { mkdirSync, readFileSync, renameSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'

export function preferencesPath({ home = os.homedir(), herdrConfigPath = process.env.HERDR_CONFIG_PATH } = {}) {
  const configPath = herdrConfigPath ? path.resolve(herdrConfigPath) : path.join(home, '.config', 'herdr', 'config.toml')
  return path.join(path.dirname(configPath), 'plugins', 'config', 'soniiit.github-ops', 'preferences.json')
}

export function readPreferences({ filePath = preferencesPath() } = {}) {
  try {
    const value = JSON.parse(readFileSync(filePath, 'utf8'))
    return value && typeof value === 'object' && !Array.isArray(value) ? value : {}
  } catch {
    return {}
  }
}

export function saveDefaultAgentKind(kind, { filePath = preferencesPath() } = {}) {
  const directory = path.dirname(filePath)
  mkdirSync(directory, { recursive: true })
  const preferences = readPreferences({ filePath })
  preferences.defaultAgentKind = kind
  const temporaryPath = `${filePath}.tmp-${process.pid}-${Date.now()}`
  writeFileSync(temporaryPath, `${JSON.stringify(preferences, null, 2)}\n`, { mode: 0o600 })
  renameSync(temporaryPath, filePath)
  return preferences
}
