<script lang="ts">
	import { getDbClockNow } from '$lib/forLater'
	import { displayDate } from '$lib/utils'
	import { onMount, untrack } from 'svelte'

	interface Props {
		date: string
		agoOnlyIfRecent?: boolean
		noDate?: boolean
		isRecent?: boolean
		noSeconds?: boolean
	}

	let {
		date,
		agoOnlyIfRecent = false,
		noDate = false,
		isRecent = $bindable(true),
		noSeconds = false
	}: Props = $props()

	let computedTimeAgo: string | undefined = $state(undefined)

	let interval

	onMount(() => {
		// Update every minute for noSeconds mode, every second for regular mode
		const intervalMs = noSeconds ? 60000 : 1000
		interval = setInterval(() => {
			computeDate()
			if (!isRecent) {
				clearInterval(interval)
				interval = undefined
			}
		}, intervalMs)

		// Add explicit cleanup
		return () => {
			interval && clearInterval(interval)
		}
	})

	async function computeDate() {
		computedTimeAgo = await displayDaysAgo(date)
	}

	function isToday(someDate: Date): boolean {
		const today = getDbClockNow()
		return (
			someDate.getDate() == today.getDate() &&
			someDate.getMonth() == today.getMonth() &&
			someDate.getFullYear() == today.getFullYear()
		)
	}

	function daysAgo(someDate: Date): number {
		const today = getDbClockNow()
		return Math.floor((today.getTime() - someDate.getTime()) / 86400000)
	}

	function secondsAgo(date: Date) {
		return Math.max(0, Math.floor((getDbClockNow().getTime() - date.getTime()) / 1000))
	}

	function minutesAgo(date: Date) {
		return Math.floor(secondsAgo(date) / 60)
	}

	function hoursAgo(date: Date) {
		return Math.floor(minutesAgo(date) / 60)
	}

	function isLessThanMonthAgo(date: Date): boolean {
		const today = getDbClockNow()
		const monthAgo = new Date(today)
		monthAgo.setMonth(monthAgo.getMonth() - 1)
		return date.getTime() >= monthAgo.getTime()
	}

	async function displayDaysAgo(dateString: string): Promise<string> {
		const date = await new Date(dateString)

		// New noSeconds mode
		if (noSeconds) {
			const mins = minutesAgo(date)
			const hours = hoursAgo(date)
			const dAgo = daysAgo(date)

			// If less than an hour ago -> "x min ago"
			if (mins < 60) {
				return `${mins} min ago`
			}
			// If less than a day ago -> "x hours ago"
			else if (hours < 24) {
				return `${hours} hours ago`
			}
			// If less than a month ago -> "x days ago"
			else if (isLessThanMonthAgo(date)) {
				return `${dAgo} days ago`
			}
			// If more than a month ago -> "the mm/dd/year"
			else {
				const month = (date.getMonth() + 1).toString().padStart(2, '0')
				const day = date.getDate().toString().padStart(2, '0')
				const year = date.getFullYear()
				return `the ${month}/${day}/${year}`
			}
		}

		// Original mode
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
			} else if (dAgo > 7 && !noDate) {
				return `${dAgo + 1} days ago at ${date.toLocaleTimeString([], {
					hour: '2-digit',
					minute: '2-digit'
				})}`
			} else {
				return displayDate(dateString, false, !noDate)
			}
		}
	}
	$effect(() => {
		date && untrack(() => computeDate())
	})
</script>

{#if computedTimeAgo && (!agoOnlyIfRecent || isRecent)}
	{computedTimeAgo}
{:else}
	{displayDate(date)}
{/if}
