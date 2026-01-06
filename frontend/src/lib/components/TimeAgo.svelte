<script lang="ts">
	import { getDbClockNow } from '$lib/forLater'
	import { displayDate } from '$lib/utils'
	import { onDestroy, onMount, untrack } from 'svelte'

	interface Props {
		date: string
		agoOnlyIfRecent?: boolean
		noDate?: boolean
		isRecent?: boolean
	}

	let {
		date,
		agoOnlyIfRecent = false,
		noDate = false,
		isRecent = $bindable(true)
	}: Props = $props()

	let computedTimeAgo: string | undefined = $state(undefined)

	let interval

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

	async function displayDaysAgo(dateString: string): Promise<string> {
		const date = await new Date(dateString)
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
