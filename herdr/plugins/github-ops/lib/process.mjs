import { spawnSync } from 'node:child_process'

export function run(command, args, { cwd, env = process.env, input, allowFailure = false } = {}) {
  const result = spawnSync(command, args, { cwd, env, input, encoding: 'utf8' })
  if (result.error) throw new Error(`${command}: ${result.error.message}`)
  const output = { status: result.status ?? 1, stdout: result.stdout ?? '', stderr: result.stderr ?? '' }
  if (!allowFailure && output.status !== 0) {
    throw new Error((output.stderr || output.stdout || `${command} exited ${output.status}`).trim())
  }
  return output
}

export function runInteractive(command, args, { cwd, env = process.env } = {}) {
  const result = spawnSync(command, args, { cwd, env, stdio: 'inherit' })
  if (result.error) throw new Error(`${command}: ${result.error.message}`)
  if (result.status !== 0) throw new Error(`${command} exited ${result.status ?? 1}`)
}

export function jsonCommand(command, args, options = {}) {
  const { stdout } = run(command, args, options)
  try {
    return JSON.parse(stdout)
  } catch (error) {
    throw new Error(`${command} returned invalid JSON: ${error.message}`)
  }
}
