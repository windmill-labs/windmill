import { get } from 'svelte/store'
import { dbClockDrift } from './stores'
import { JobService } from './gen'

function subtractSeconds(date: Date, seconds: number): Date {
	date.setSeconds(date.getSeconds() - seconds)
	return date
}

function adjustDate(drift: number): Date {
	return new Date(Date.now() + drift)
}

export async function computeDrift() {
	const now = Date.now()
	const dbClock = await JobService.getDbClock()
	dbClockDrift.set(dbClock - now)
}

export function forLater(scheduledString: string): boolean {
	return getDbClockNow() < subtractSeconds(new Date(scheduledString), 5)
}

export function getDbClockNow() {
	let drift = get(dbClockDrift)
	if (drift == undefined) {
		computeDrift()
		drift = 0
	}
	return adjustDate(drift)
}
