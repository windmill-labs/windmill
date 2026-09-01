<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { forLater } from '$lib/forLater'
	import {
		Ban,
		Calendar,
		Check,
		FastForward,
		Hourglass,
		Play,
		ShieldQuestion,
		Wrench,
		X
	} from 'lucide-svelte'
	import type { Job } from '$lib/gen'

	interface Props {
		job: Job
		isExternal?: boolean
		roundedFull?: boolean
		/** Icon size in px, and the padding around it. Defaults are the runs page's; the chat's
		 * tool rows ask for a smaller one, since a 30px badge would set the height of a row of
		 * 11px text. */
		size?: number
		badgeClass?: string
	}

	let {
		job,
		isExternal = false,
		roundedFull = false,
		size = 14,
		badgeClass = undefined
	}: Props = $props()
</script>

<div class="flex items-center justify-start">
	{#if isExternal}
		<Badge color="gray" {roundedFull} baseClass={roundedFull ? '' : '!px-1.5'}
			class={badgeClass}>
			<ShieldQuestion size={size} />
		</Badge>
	{:else if job.canceled && 'success' in job}
		<Badge color="gray" {roundedFull} baseClass={roundedFull ? '' : '!px-1.5'}
			class={badgeClass} title="Canceled">
			<Ban size={size} />
		</Badge>
	{:else if 'success' in job && job.success}
		{#if job.is_skipped}
			<Badge color="green" {roundedFull} baseClass={roundedFull ? '' : ''}
				class={badgeClass}>
				<FastForward size={size} />
			</Badge>
		{:else}
			<Badge color="green" {roundedFull} baseClass={roundedFull ? '' : '!px-1.5'}
			class={badgeClass}>
				<Check size={size} />
			</Badge>
		{/if}
	{:else if 'success' in job && job.resolved}
		<Badge
			color="orange"
			{roundedFull}
			baseClass={roundedFull ? '' : '!px-1.5'}
			class={badgeClass}
			title="Failed, marked as resolved"
		>
			<Wrench size={size} />
		</Badge>
	{:else if 'success' in job}
		<Badge color="red" {roundedFull} baseClass={roundedFull ? '' : '!px-1.5'}
			class={badgeClass}>
			<X size={size} />
		</Badge>
	{:else if 'running' in job && job.running && job.suspend}
		<Badge color="violet" {roundedFull} baseClass={roundedFull ? '' : '!px-1.5'}
			class={badgeClass} title="Suspended">
			<Hourglass size={size} />
		</Badge>
	{:else if 'running' in job && job.running}
		<Badge color="yellow" {roundedFull} baseClass={roundedFull ? '' : '!px-1.5'}
			class={badgeClass}>
			<Play size={size} />
		</Badge>
	{:else if job && 'running' in job && job.scheduled_for && forLater(job.scheduled_for)}
		<Badge color="blue" {roundedFull} baseClass={roundedFull ? '' : '!px-1.5'}
			class={badgeClass}>
			<Calendar size={size} />
		</Badge>
	{:else}
		<Badge {roundedFull} baseClass={roundedFull ? '' : '!px-1.5'} class={badgeClass}>
			<Hourglass size={size} />
		</Badge>
	{/if}
</div>
