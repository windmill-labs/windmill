<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { forLater } from '$lib/forLater'
	import {
		Calendar,
		Check,
		FastForward,
		Hourglass,
		Play,
		ShieldQuestion,
		X
	} from 'lucide-svelte'
	import type { Job } from '$lib/gen'

	interface Props {
		job: Job
		isExternal?: boolean
		roundedFull?: boolean
	}

	let { job, isExternal = false, roundedFull = false }: Props = $props()
</script>

<div class="flex items-center justify-start">
	{#if isExternal}
		<Badge color="gray" {roundedFull} baseClass={roundedFull ? '' : '!px-1.5'}>
			<ShieldQuestion size={14} />
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
	{:else if 'success' in job}
		<Badge color="red" {roundedFull} baseClass={roundedFull ? '' : '!px-1.5'}>
			<X size={14} />
		</Badge>
	{:else if 'running' in job && job.running && job.suspend}
		<Badge color="violet" {roundedFull} baseClass={roundedFull ? '' : '!px-1.5'} title="Suspended">
			<Hourglass size={14} />
		</Badge>
	{:else if 'running' in job && job.running}
		<Badge color="yellow" {roundedFull} baseClass={roundedFull ? '' : '!px-1.5'}>
			<Play size={14} />
		</Badge>
	{:else if job && 'running' in job && job.scheduled_for && forLater(job.scheduled_for)}
		<Badge color="blue" {roundedFull} baseClass={roundedFull ? '' : '!px-1.5'}>
			<Calendar size={14} />
		</Badge>
	{:else if job.canceled}
		<Badge color="red" {roundedFull} baseClass={roundedFull ? '' : '!px-1.5'}>
			<Hourglass size={14} />
		</Badge>
	{:else}
		<Badge {roundedFull} baseClass={roundedFull ? '' : '!px-1.5'}>
			<Hourglass size={14} />
		</Badge>
	{/if}
</div>
