<script lang="ts">
	import SchemaPickerRow from './SchemaPickerRow.svelte'
	import { ExternalLink } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'

	const operatingWorkspace = useOperatingWorkspace()

	interface Props {
		job: any
		payloadData?: any | undefined
		hovering?: boolean
		showAuthor?: boolean
		isPreview?: boolean
		placement?: 'bottom-start' | 'top-start' | 'bottom-end' | 'top-end'
		viewerOpen?: boolean
		limitPayloadSize?: boolean
		/** How the run started, shown before the author. */
		icon?: { icon: any; title: string } | undefined
		/** Shown instead of the author. */
		label?: string | undefined
		/** A run listed inside the one above it. */
		nested?: boolean
		/** Shown instead of the author or label. */
		aside?: import('svelte').Snippet
	}

	let {
		job,
		payloadData = undefined,
		hovering = false,
		showAuthor = false,
		isPreview = false,
		placement = 'bottom-start',
		viewerOpen = false,
		limitPayloadSize = false,
		icon = undefined,
		label = undefined,
		nested = false,
		aside
	}: Props = $props()
</script>

<SchemaPickerRow
	{payloadData}
	date={job.created_at}
	{hovering}
	{placement}
	{viewerOpen}
	on:openChange
	{limitPayloadSize}
>
	{#snippet start()}
		<div class={twMerge('center-center relative', nested && 'pl-3')}>
			{#if job.running}
				<div
					class={twMerge(
						'rounded-full w-2 h-2 animate-pulse',
						job.suspended ? 'bg-violet-400' : 'bg-orange-400'
					)}
					title={job.suspended ? 'Suspended' : 'Running'}
				></div>
			{:else}
				<div
					class="rounded-full w-2 h-2 {job.success ? 'bg-green-400' : 'bg-red-400'}"
					title={job.success ? 'Success' : 'Failed'}
				></div>
			{/if}
			{#if isPreview}
				<span
					class="absolute -top-1.5 right-0.5 text-[8px] font-bold leading-none text-tertiary"
					title="Preview run">P</span
				>
			{/if}
		</div>
	{/snippet}
	{#snippet extra()}
		<div class="flex flex-row items-center gap-2">
			{#if icon}
				<span class="center-center text-tertiary" title={icon.title}>
					<icon.icon size={14} />
				</span>
			{/if}
			{#if aside}
				<div class="flex w-28">
					{@render aside()}
				</div>
			{:else if label != undefined}
				<span class="text-secondary px-2 w-28 truncate" title={label}>
					{label}
				</span>
			{:else if showAuthor}
				<span class="text-secondary px-2 w-28 truncate" title={job.created_by}>
					{job.created_by}
				</span>
			{/if}
			<div class="center-center">
				<a
					target="_blank"
					href="/run/{job.id}?workspace={$operatingWorkspace}"
					class="text-right float-right text-secondary px-2"
					title="See run detail in a new tab"
				>
					<ExternalLink size={16} />
				</a>
			</div>
		</div>
	{/snippet}
</SchemaPickerRow>
