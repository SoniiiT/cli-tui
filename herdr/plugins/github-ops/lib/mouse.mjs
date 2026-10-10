export class SgrMouseParser {
  buffer = ''

  push(chunk) {
    const input = this.buffer + (Buffer.isBuffer(chunk) ? chunk.toString('utf8') : String(chunk))
    const events = []
    let reportCount = 0
    let cursor = 0

    while (cursor < input.length) {
      const start = input.indexOf('\u001b[<', cursor)
      if (start < 0) break

      const match = /^\u001b\[<(\d+);(\d+);(\d+)([Mm])/.exec(input.slice(start))
      if (!match) {
        this.buffer = input.slice(start).slice(-64)
        return { events, reportCount }
      }

      reportCount++
      const buttonCode = Number(match[1])
      if (match[4] === 'M' && (buttonCode & 3) === 0 && (buttonCode & 96) === 0) {
        events.push({ x: Number(match[2]) - 1, y: Number(match[3]) - 1 })
      }
      cursor = start + match[0].length
    }

    this.buffer = input.slice(-2)
    return { events, reportCount }
  }
}
