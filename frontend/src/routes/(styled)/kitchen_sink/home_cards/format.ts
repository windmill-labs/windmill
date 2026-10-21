import type { Entry } from './model'

const MIN = 60_000

/** The compact time a card shows: "now", "4m", "2h", "3d", "2w". */
export function ago(at: number, now: number): string {
	const mins = Math.max(0, Math.floor((now - at) / MIN))
	if (mins < 1) return 'now'
	if (mins < 60) return `${mins}m`
	if (mins < 1440) return `${Math.floor(mins / 60)}h`
	if (mins < 14 * 1440) return `${Math.floor(mins / 1440)}d`
	return `${Math.floor(mins / (7 * 1440))}w`
}

/** The hover text of that time. */
export function editedTooltip(at: number, now: number): string {
	const mins = Math.max(0, Math.floor((now - at) / MIN))
	const unit = (n: number, word: string) => `${n} ${word}${n === 1 ? '' : 's'} ago`
	const when =
		mins < 1
			? 'just now'
			: mins < 60
				? unit(mins, 'minute')
				: mins < 1440
					? unit(Math.floor(mins / 60), 'hour')
					: unit(Math.floor(mins / 1440), 'day')
	return `Edited by you ${when}`
}

/** An item's title in two parts: a summary is all `name`; a bare path keeps its last segment as
 * `name` and the folder before it as `lead`, which the quieter variants grey out. */
export function titleParts(entry: Entry): { lead: string; name: string } {
	if (entry.summary) return { lead: '', name: entry.summary }
	const cut = entry.path.lastIndexOf('/')
	return { lead: entry.path.slice(0, cut + 1), name: entry.path.slice(cut + 1) }
}

export function titleOf(entry: Entry): string {
	return entry.summary || entry.path
}

export function kindLabel(entry: Entry): string {
	switch (entry.kind) {
		case 'raw_app':
			return 'Raw app'
		case 'app':
			return 'App'
		case 'flow':
			return 'Flow'
		case 'agent':
			return 'Agent'
		default:
			return 'Script'
	}
}

export function keyOf(entry: Entry): string {
	return `${entry.kind}/${entry.path}`
}
