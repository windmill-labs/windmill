<script lang="ts">
	import { Lock, Timer } from 'lucide-svelte'
	import { displayDate, msToCompactDuration } from '$lib/utils'
	import type { GraphModuleState } from '../../model'

	interface Props {
		reason: NonNullable<GraphModuleState['waiting_reason']>
		until: Date | string
		concurrencyKey?: string
	}

	let { reason, until, concurrencyKey }: Props = $props()

	let now = $state(Date.now())
	$effect(() => {
		const id = setInterval(() => (now = Date.now()), 1000)
		return () => clearInterval(id)
	})

	let left = $derived(Math.max(new Date(until).getTime() - now, 0))
	let title = $derived(
		reason === 'sleep'
			? `Sleeping after the previous step until ${displayDate(until, true)}`
			: `Held back by its concurrency limit${concurrencyKey ? ` (key ${concurrencyKey})` : ''}, next check by ${displayDate(until, true)}`
	)
</script>

<div
	class="absolute -left-1 top-[calc(100%+3px)] z-10 flex items-center gap-1 px-1 h-5 text-2xs text-secondary"
	{title}
>
	{#if reason === 'sleep'}
		<Timer size={12} class="text-orange-500" />
		Sleeping · {msToCompactDuration(left, 0)}
	{:else}
		<Lock size={12} class="text-orange-500" />
		Waiting for a concurrency slot
	{/if}
</div>
