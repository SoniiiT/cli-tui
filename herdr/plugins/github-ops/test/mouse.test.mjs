import assert from 'node:assert/strict'
import test from 'node:test'
import { SgrMouseParser } from '../lib/mouse.mjs'

test('parses primary mouse presses with zero-based terminal coordinates across chunks', () => {
  const parser = new SgrMouseParser()

  assert.deepEqual(parser.push('\u001b[<0;12;'), { events: [], reportCount: 0 })
  assert.deepEqual(parser.push('8M'), { events: [{ x: 11, y: 7 }], reportCount: 1 })
})

test('ignores mouse release, non-primary buttons and wheel reports', () => {
  const parser = new SgrMouseParser()
  const result = parser.push('\u001b[<0;3;4m\u001b[<2;5;6M\u001b[<64;7;8M')

  assert.equal(result.reportCount, 3)
  assert.deepEqual(result.events, [])
})

test('extracts multiple primary presses from one input chunk', () => {
  const parser = new SgrMouseParser()
  const result = parser.push('\u001b[<0;2;3M\u001b[<0;9;10M')

  assert.deepEqual(result.events, [{ x: 1, y: 2 }, { x: 8, y: 9 }])
  assert.equal(result.reportCount, 2)
})
