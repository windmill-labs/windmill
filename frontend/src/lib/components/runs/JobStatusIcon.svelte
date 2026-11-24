<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { forLater } from '$lib/forLater'
	import {
		Ban,
		Calendar,
		Check,
		FastForward,
		Gauge,
		Hourglass,
		Moon,
		Play,
		ShieldQuestion,
		Wrench,
		X
	} from 'lucide-svelte'
	import type { Job } from '$lib/gen'
	import { isSuspended } from '../agentRequest'

	interface Props {
		job: Job
		isExternal?: boolean
		roundedFull?: boolean
		/** Its concurrency limit holds the job back. */
		heldByConcurrency?: boolean
		/** Why the step the flow is at has not started. */
		stepWaiting?: 'sleep' | 'concurrency'
	}

	let {
		job,
		isExternal = false,
		roundedFull = false,
		heldByConcurrency = false,
		stepWaiting
	}: Props = $props()
</script>

<div class="flex items-center justify-start">
	{#if isExternal}
		<Badge color="gray" {roundedFull} baseClass={roundedFull ? '' : '!px-1.5'}>
			<ShieldQuestion size={14} />
		</Badge>
	{:else if job.canceled && 'success' in job}
		<Badge
			color="gray"
			{roundedFull}
			baseClass={roundedFull ? '' : '!px-1.5'}
			title={job.resolved ? 'Canceled, marked as resolved' : 'Canceled'}
		>
			<Ban size={14} />
		</Badge>
	{:else if 'success' in job && job.success}
		{#if job.is_skipped}
			<Badge color="green" {roundedFull} baseClass={roundedFull ? '' : ''}>
				<FastForward size={14} />
			</Badge>
		{:else}
			<Badge color="green" {roundedFull} baseClass={roundedFull ? '' : '!px-1.5'}>
				<Check size={14} />
			</Badge>
		{/if}
	{:else if 'success' in job && job.resolved}
		<Badge
			color="orange"
			{roundedFull}
			baseClass={roundedFull ? '' : '!px-1.5'}
			title="Failed, marked as resolved"
		>
			<Wrench size={14} />
		</Badge>
	{:else if 'success' in job}
		<Badge color="red" {roundedFull} baseClass={roundedFull ? '' : '!px-1.5'}>
			<X size={14} />
		</Badge>
	{:else if 'running' in job && job.running && isSuspended(job)}
		<Badge color="violet" {roundedFull} baseClass={roundedFull ? '' : '!px-1.5'} title="Suspended">
			<Hourglass size={14} />
		</Badge>
	{:else if 'running' in job && job.running && stepWaiting === 'sleep'}
		<Badge {roundedFull} baseClass={roundedFull ? '' : '!px-1.5'} title="Sleeping">
			<Moon size={14} />
		</Badge>
	{:else if 'running' in job && job.running && stepWaiting === 'concurrency'}
		<Badge
			color="orange"
			{roundedFull}
			baseClass={roundedFull ? '' : '!px-1.5'}
			title="Step held by concurrency limit"
		>
			<Gauge size={14} />
		</Badge>
	{:else if 'running' in job && job.running}
		<Badge color="yellow" {roundedFull} baseClass={roundedFull ? '' : '!px-1.5'}>
			<Play size={14} />
		</Badge>
	{:else if 'running' in job && heldByConcurrency}
		<Badge
			color="orange"
			{roundedFull}
			baseClass={roundedFull ? '' : '!px-1.5'}
			title="Held by concurrency limit"
		>
			<Gauge size={14} />
		</Badge>
	{:else if job && 'running' in job && job.scheduled_for && forLater(job.scheduled_for)}
		<Badge color="blue" {roundedFull} baseClass={roundedFull ? '' : '!px-1.5'}>
			<Calendar size={14} />
		</Badge>
	{:else}
		<Badge {roundedFull} baseClass={roundedFull ? '' : '!px-1.5'}>
			<Hourglass size={14} />
		</Badge>
	{/if}
</div>
