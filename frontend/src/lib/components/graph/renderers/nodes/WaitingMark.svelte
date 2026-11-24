<script lang="ts">
	import { Gauge, Moon } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { displayDate, msToCompactDuration } from '$lib/utils'
	import type { GraphModuleState } from '../../model'

	interface Props {
		reason: NonNullable<GraphModuleState['waiting_reason']>
		until: Date | string | undefined
		concurrencyKey?: string
		heldSince?: string
		onclick: () => void
	}

	let { reason, until, concurrencyKey, heldSince, onclick }: Props = $props()

	let now = $state(Date.now())
	$effect(() => {
		const id = setInterval(() => (now = Date.now()), 1000)
		return () => clearInterval(id)
	})

	let left = $derived(Math.max(new Date(until ?? now).getTime() - now, 0))
	let held = $derived(heldSince ? Math.max(now - new Date(heldSince).getTime(), 0) : undefined)
	let title = $derived(
		reason === 'sleep'
			? `Sleeping after the previous step until ${displayDate(until, true)}`
			: `Held by its concurrency limit${held != undefined ? ` for ${msToCompactDuration(held, 0)}` : ''}${concurrencyKey ? ` (key ${concurrencyKey})` : ''}, next check by ${displayDate(until, true)}`
	)
</script>

<!-- A sleep is the time before this step starts, so it sits above it; finished steps show how
     long they ran at the other end of that edge. -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="nodrag nopan absolute -left-1 z-10 {reason === 'sleep'
		? 'bottom-[calc(100%+3px)]'
		: 'top-[calc(100%+3px)]'}"
	onpointerdown={(e) => e.stopPropagation()}
>
	<Button
		variant="subtle"
		unifiedSize="2xs"
		startIcon={{
			icon: reason === 'sleep' ? Moon : Gauge,
			classes: reason === 'sleep' ? 'text-blue-500' : 'text-orange-500'
		}}
		btnClasses="text-2xs text-secondary gap-1"
		{title}
		onClick={(e) => {
			e?.stopPropagation()
			onclick()
		}}
	>
		{#if reason === 'sleep'}
			Sleeping · {msToCompactDuration(left, 0)}
		{:else}
			Held by concurrency limit{held != undefined ? ` · ${msToCompactDuration(held, 0)}` : ''}
		{/if}
	</Button>
</div>
