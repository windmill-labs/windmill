<script lang="ts">
	import { JobService } from '$lib/gen'
	import { dbClockDrift } from '$lib/stores'
	import { displayDate } from '$lib/utils'
	import { onDestroy, onMount } from 'svelte'

	export let date: string
	export let withDate: boolean = false
	export let agoOnlyIfRecent: boolean = false

	let computedTimeAgo: string | undefined = undefined

	let isRecent = true
	let interval
	computeDate()

	onMount(() => {
		interval = setInterval(() => {
			computeDate()
			if (!isRecent) {
				clearInterval(interval)
				interval = undefined
			}
		}, 1000)
	})

	onDestroy(() => {
		interval && clearInterval(interval)
	})

	async function computeDate() {
		computedTimeAgo = await displayDaysAgo(date)
	}

	function isToday(someDate: Date): boolean {
		const today = new Date()
		return (
			someDate.getDate() == today.getDate() &&
			someDate.getMonth() == today.getMonth() &&
			someDate.getFullYear() == today.getFullYear()
		)
	}

	function daysAgo(someDate: Date): number {
		const today = new Date()
		return Math.floor((today.getTime() - someDate.getTime()) / 86400000)
	}

	function secondsAgo(date: Date) {
		return Math.max(0, Math.floor((new Date().getTime() - date.getTime()) / 1000))
	}

	async function computeDrift() {
		const storeDrift = $dbClockDrift
		if (storeDrift) {
			return storeDrift
		} else {
			const now = Date.now()
			const dbClock = await JobService.getDbClock()
			const drift = now - dbClock
			$dbClockDrift = drift
			return drift
		}
	}

	async function adjustDate(date: Date): Promise<Date> {
		return new Date(date.getTime() + (await computeDrift()))
	}
	async function displayDaysAgo(dateString: string): Promise<string> {
		const date = await adjustDate(new Date(dateString))
		const nbSecondsAgo = secondsAgo(date)
		if (nbSecondsAgo < 600) {
			return `${nbSecondsAgo}s ago`
		} else if (isToday(date)) {
			isRecent = false
			return `today at ${date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`
		} else {
			isRecent = false
			let dAgo = daysAgo(date)
			if (dAgo == 0) {
				return `yesterday at ${date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`
			} else if (dAgo > 7) {
				return `${dAgo + 1} days ago at ${date.toLocaleTimeString([], {
					hour: '2-digit',
					minute: '2-digit'
				})}`
			} else {
				return !withDate ? displayDate(dateString) : ''
			}
		}
	}
</script>

{#if withDate}
	{displayDate(date)}
{/if}
{#if computedTimeAgo && (!agoOnlyIfRecent || isRecent)}
	{computedTimeAgo}
{/if}
