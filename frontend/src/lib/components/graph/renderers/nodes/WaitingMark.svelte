<script lang="ts">
	import { Gauge, Hourglass, Moon } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { useApprovalCount } from '../../approvalCount.svelte'
	import { displayDate, msToCompactDuration } from '$lib/utils'
	import type { GraphModuleState } from '../../model'

	type Props = { onclick: () => void } & (
		| {
				reason: NonNullable<GraphModuleState['waiting_reason']>
				until: Date | string | undefined
				concurrencyKey?: string
				heldSince?: string
		  }
		| {
				reason: 'approval'
				approvalWait: NonNullable<GraphModuleState['approval_wait']>
		  }
	)

	let props: Props = $props()

	let now = $state(Date.now())
	$effect(() => {
		if (props.reason === 'approval') return
		const id = setInterval(() => (now = Date.now()), 1000)
		return () => clearInterval(id)
	})

	const approval = useApprovalCount(() =>
		props.reason === 'approval' ? props.approvalWait : undefined
	)

	let label = $derived.by(() => {
		if (props.reason === 'approval') return approval.label
		if (props.reason === 'sleep')
			return `Sleeping · ${msToCompactDuration(Math.max(new Date(props.until ?? now).getTime() - now, 0), 0)}`
		const held = props.heldSince
			? msToCompactDuration(Math.max(now - new Date(props.heldSince).getTime(), 0), 0)
			: undefined
		return `Held by concurrency limit${held ? ` · ${held}` : ''}`
	})
	let title = $derived.by(() => {
		if (props.reason === 'approval') return 'The run waits for this step to be approved'
		if (props.reason === 'sleep')
			return `Sleeping after the previous step until ${displayDate(props.until, true)}`
		return `Held by its concurrency limit${props.concurrencyKey ? ` (key ${props.concurrencyKey})` : ''}, next check by ${displayDate(props.until, true)}`
	})
</script>

<!-- What keeps a step from starting sits above it; finished steps show how long they ran at the
     other end of that edge. -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="nodrag nopan absolute -left-1 bottom-[calc(100%+3px)] z-10"
	onpointerdown={(e) => e.stopPropagation()}
>
	<Button
		variant="subtle"
		unifiedSize="2xs"
		startIcon={{
			icon: props.reason === 'approval' ? Hourglass : props.reason === 'sleep' ? Moon : Gauge,
			classes:
				props.reason === 'approval'
					? 'text-violet-500'
					: props.reason === 'concurrency'
						? 'text-orange-500'
						: ''
		}}
		btnClasses="text-2xs text-secondary gap-1"
		{title}
		onClick={(e) => {
			e?.stopPropagation()
			props.onclick()
		}}
	>
		{label}
	</Button>
</div>
