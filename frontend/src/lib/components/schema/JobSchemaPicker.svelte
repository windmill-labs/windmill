<script lang="ts">
	import SchemaPickerRow from './SchemaPickerRow.svelte'
	import { workspaceStore } from '$lib/stores'
	import { ExternalLink } from 'lucide-svelte'
	import { base } from '$lib/base'

	export let job: any
	export let payloadData: any | undefined = undefined
	export let hovering = false
	export let showAuthor = false
	export let placement: 'bottom-start' | 'top-start' | 'bottom-end' | 'top-end' = 'bottom-start'
	export let viewerOpen = false
	export let limitPayloadSize = false
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
	<svelte:fragment slot="start">
		<div class="center-center">
			<div
				class="rounded-full w-2 h-2 {job.success ? 'bg-green-400' : 'bg-red-400'}"
				title={job.success ? 'Success' : 'Failed'}
			></div>
		</div>
	</svelte:fragment>
	<svelte:fragment slot="extra">
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
	</svelte:fragment>
</SchemaPickerRow>
