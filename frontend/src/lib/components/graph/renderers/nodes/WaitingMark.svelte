<script lang="ts">
	import { Gauge, Hourglass, Moon } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { JobService } from '$lib/gen'
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
				/** The approval step's run and its id. */
				workspace: string | undefined
				jobId: string
				stepId: string
				required: number
		  }
	)

	let props: Props = $props()

	let now = $state(Date.now())
	$effect(() => {
		if (props.reason === 'approval') return
		const id = setInterval(() => (now = Date.now()), 1000)
		return () => clearInterval(id)
	})

	// Approvals coming in do not change the run until the last one, so the count is read on its
	// own, and only when there is more than one to count.
	let approvals = $state<number | undefined>(undefined)
	$effect(() => {
		if (props.reason !== 'approval' || props.required <= 1 || !props.workspace) return
		const args = { workspace: props.workspace, jobId: props.jobId, stepId: props.stepId }
		const read = () =>
			JobService.getApprovalStepDetail(args)
				.then((d) => (approvals = d.approvals.length))
				.catch(() => {})
		read()
		const id = setInterval(read, 10000)
		return () => clearInterval(id)
	})

	let label = $derived.by(() => {
		if (props.reason === 'approval')
			return `Waiting for approval${approvals ? ` · ${approvals} of ${props.required}` : ''}`
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

<!-- What keeps a step from starting sits above it; an approval, which comes once its step is
     done, sits below it. -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="nodrag nopan absolute -left-1 z-10 {props.reason === 'approval'
		? 'top-[calc(100%+3px)]'
		: 'bottom-[calc(100%+3px)]'}"
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
		btnClasses="text-2xs gap-1 {props.reason === 'approval'
			? 'text-violet-600 dark:text-violet-400'
			: 'text-secondary'}"
		{title}
		onClick={(e) => {
			e?.stopPropagation()
			props.onclick()
		}}
	>
		{label}
	</Button>
</div>
