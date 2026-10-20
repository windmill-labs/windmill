<script lang="ts">
	import SchemaPickerRow from './SchemaPickerRow.svelte'
	import { ExternalLink, Workflow } from 'lucide-svelte'
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
		<div class="center-center relative">
			<div
				class="rounded-full w-2 h-2 {job.success ? 'bg-green-400' : 'bg-red-400'}"
				title={job.success ? 'Success' : 'Failed'}
			></div>
			{#if isPreview}
				<span
					class="absolute -top-1.5 right-0.5 text-[8px] font-bold leading-none text-tertiary"
					title="Preview run">P</span
				>
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
			{#if job.parent_job}
				<div class="center-center">
					<a
						target="_blank"
						href="/run/{job.parent_job}?workspace={$operatingWorkspace}"
						class="text-secondary px-1"
						title="Ran as a flow step: see the parent flow run in a new tab"
					>
						<Workflow size={16} />
					</a>
				</div>
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
