/** A naive value's digits are its time: `Date` would read a zone-less date-time as local time */
export function parseDateTime(value: string, timezone: 'naive' | 'local'): Date {
	const zoneless = /^\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}(:\d{2}(\.\d+)?)?$/.test(value)
	return new Date(timezone === 'naive' && zoneless ? value.replace(' ', 'T') + 'Z' : value)
}
