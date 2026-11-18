/** A running clock's label: whole seconds only (`0s`, `1s`, … `1m 5s`, `1h 2m`), so it reads as a
 * ticking clock rather than a fractional stopwatch. Settled durations keep `msToReadableTime`. */
export function formatElapsed(ms: number): string {
	const total = Math.max(0, Math.floor(ms / 1000))
	if (total < 60) return `${total}s`
	const m = Math.floor(total / 60)
	const s = total % 60
	if (m < 60) return s === 0 ? `${m}m` : `${m}m ${s}s`
	const h = Math.floor(m / 60)
	const rm = m % 60
	return rm === 0 ? `${h}h` : `${h}h ${rm}m`
}
