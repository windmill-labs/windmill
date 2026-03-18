<script lang="ts">
	import SchemaPickerRow from './SchemaPickerRow.svelte'
	import { workspaceStore } from '$lib/stores'
	import { ExternalLink } from 'lucide-svelte'
	import { base } from '$lib/base'

	interface Props {
		job: any
		payloadData?: any | undefined
		hovering?: boolean
		showAuthor?: boolean
		isPreview?: boolean
		placement?: 'bottom-start' | 'top-start' | 'bottom-end' | 'top-end'
		viewerOpen?: boolean
		limitPayloadSize?: boolean
	}

	let {
		job,
		payloadData = undefined,
		hovering = false,
		showAuthor = false,
		isPreview = false,
		placement = 'bottom-start',
		viewerOpen = false,
		limitPayloadSize = false
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
		<div class="center-center">
			{#if isPreview}
				<span
					class="text-2xs font-medium px-1 rounded bg-indigo-100 text-indigo-700 dark:bg-indigo-900 dark:text-indigo-300"
					title="Preview run">P</span
				>
			{:else}
				<div
					class="rounded-full w-2 h-2 {job.success ? 'bg-green-400' : 'bg-red-400'}"
					title={job.success ? 'Success' : 'Failed'}
				></div>
			{/if}
		</div>
	{/snippet}
	{#snippet extra()}
		<div class="flex flex-row gap-2">
			{#if showAuthor}
				<span class="text-secondary px-2 w-28 truncate" title={job.created_by}>
					{job.created_by}
				</span>
			{/if}
			<div class="center-center">
				<a
					target="_blank"
					href="{base}/run/{job.id}?workspace={$workspaceStore}"
					class="text-right float-right text-secondary px-2"
					title="See run detail in a new tab"
				>
					<ExternalLink size={16} />
				</a>
			</div>
		</div>
	{/snippet}
</SchemaPickerRow>
