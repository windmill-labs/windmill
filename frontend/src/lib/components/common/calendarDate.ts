export interface CalendarDate {
	day: number | null
	month: number | null // 1-indexed
	year: number | null
	hour: number | null
	minute: number | null
}

export interface CalendarRange {
	start: CalendarDate
	end: CalendarDate
}

export function fromCalendarDate(cd: CalendarDate | null | undefined): Date | null {
	if (calendarDateIsNull(cd)) return null
	const now = new Date()
	return new Date(
		cd?.year ?? now.getFullYear(),
		(cd?.month ?? now.getMonth() + 1) - 1,
		cd?.day ?? now.getDate(),
		cd?.hour ?? 0,
		cd?.minute ?? 0
	)
}
export function toCalendarDate(date: Date | null | undefined): CalendarDate {
	if (!date) {
		return { day: null, month: null, year: null, hour: null, minute: null }
	}
	return {
		day: date.getDate(),
		month: date.getMonth() + 1,
		year: date.getFullYear(),
		hour: date.getHours(),
		minute: date.getMinutes()
	}
}
function calendarDateIsNull(cd: CalendarDate | undefined | null): boolean {
	if (!cd) return true
	return cd.day == null || cd.month == null || cd.year == null
}
