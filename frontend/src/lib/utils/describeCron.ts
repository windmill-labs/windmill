import { formatCron } from '$lib/utils'

const DAY_NAMES = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday']
const DAY_ALIASES: Record<string, number> = {
	SUN: 0,
	MON: 1,
	TUE: 2,
	WED: 3,
	THU: 4,
	FRI: 5,
	SAT: 6
}

const isNum = (f: string) => /^\d+$/.test(f)
const everyN = (f: string) => /^\*\/(\d+)$/.exec(f)?.[1]

function time(hour: string, minute: string): string {
	return `${Number(hour)}:${minute.padStart(2, '0')}`
}

function ordinal(n: number): string {
	const rest = n % 100
	if (rest >= 11 && rest <= 13) return `${n}th`
	return `${n}${['th', 'st', 'nd', 'rd'][n % 10] ?? 'th'}`
}

function dayNumber(token: string): number | undefined {
	const t = token.toUpperCase()
	if (t in DAY_ALIASES) return DAY_ALIASES[t]
	if (!isNum(t)) return undefined
	const n = Number(t)
	return n >= 0 && n <= 7 ? n % 7 : undefined
}

/** `1-5`, `MON-FRI`, `1,3,5`, `6` → sorted day numbers; undefined for anything else. */
function days(field: string): number[] | undefined {
	const out = new Set<number>()
	for (const part of field.split(',')) {
		const [from, to] = part.split('-')
		const a = dayNumber(from)
		if (a === undefined) return undefined
		if (to === undefined) {
			out.add(a)
			continue
		}
		const b = dayNumber(to)
		if (b === undefined || b < a) return undefined
		for (let d = a; d <= b; d++) out.add(d)
	}
	return [...out].sort()
}

function listDays(ds: number[]): string {
	if (ds.join() === '1,2,3,4,5') return 'Weekdays'
	if (ds.join() === '0,6') return 'Weekends'
	if (ds.length === 1) return `${DAY_NAMES[ds[0]]}s`
	return ds.map((d) => DAY_NAMES[d].slice(0, 3)).join('/')
}

/**
 * A short plain-English reading of a Windmill cron expression — "Hourly",
 * "Daily, 4:00", "Mondays, 9:30" — or undefined when it is not one
 * of the common shapes, so the caller can show the expression itself.
 *
 * Windmill crons have six fields, seconds first; a five-field entry is padded
 * at the end the way the schedule editor saves it (`formatCron`).
 */
export function describeCron(cron: string): string | undefined {
	const fields = formatCron(cron.trim()).split(/\s+/)
	if (fields.length !== 6) return undefined
	const [sec, min, hour, dom, month, dow] = fields
	if (month !== '*') return undefined

	const anyDay = dom === '*' && (dow === '*' || dow === '?')
	if (anyDay && min === '*' && hour === '*') {
		if (sec === '*') return 'Every second'
		const n = everyN(sec)
		if (n) return `Every ${n}s`
	}
	if (sec !== '0') return undefined

	if (anyDay) {
		if (hour === '*') {
			if (min === '*') return 'Every minute'
			const n = everyN(min)
			if (n) return `Every ${n} min`
			if (min === '0') return 'Hourly'
			if (isNum(min)) return `Hourly at :${min.padStart(2, '0')}`
			return undefined
		}
		const n = everyN(hour)
		if (n && min === '0') return `Every ${n}h`
		if (isNum(hour) && isNum(min)) return `Daily, ${time(hour, min)}`
		return undefined
	}

	if (!isNum(hour) || !isNum(min)) return undefined
	if (dom === '*') {
		const ds = days(dow)
		return ds ? `${listDays(ds)}, ${time(hour, min)}` : undefined
	}
	if (isNum(dom) && (dow === '*' || dow === '?')) {
		return `Monthly on the ${ordinal(Number(dom))}, ${time(hour, min)}`
	}
	return undefined
}

function shortZone(timezone: string): string {
	try {
		return (
			new Intl.DateTimeFormat('en-US', { timeZone: timezone, timeZoneName: 'short' })
				.formatToParts(new Date())
				.find((p) => p.type === 'timeZoneName')?.value ?? timezone
		)
	} catch {
		return timezone
	}
}

/** `describeCron` of a schedule, naming its zone whenever the text ends on a
 * wall-clock time: "Daily, 12:00 UTC". */
export function describeSchedule(cron: string, timezone?: string): string | undefined {
	const text = describeCron(cron)
	if (!text || !timezone || !/\d:\d\d$/.test(text)) return text
	return `${text} ${shortZone(timezone)}`
}
