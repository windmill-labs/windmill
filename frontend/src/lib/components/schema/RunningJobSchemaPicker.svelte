<script lang="ts">
	import SchemaPickerRow from './SchemaPickerRow.svelte'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import { JobService } from '$lib/gen/index.js'
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import { ExternalLink } from 'lucide-svelte'
	import { base } from '$lib/base'
	import { Cell, Row } from '$lib/components/table'
	import { twMerge } from 'tailwind-merge'

	export let job: any
	export let selected = false
	export let payloadData: any | undefined = undefined
	const dispatch = createEventDispatcher()
	export let hovering = false

	let loadingArgs = true
	loadArgsFromRunningJob(job.id)

	async function loadArgsFromRunningJob(id: string | undefined) {
		if (!id) return
		payloadData = await JobService.getJobArgs({
			workspace: $workspaceStore!,
			id
		})
		loadingArgs = false
	}
</script>

{#if loadingArgs}
	<Cell>
		<Skeleton layout={[[1]]} />
	</Cell>
	<Cell>
		<Skeleton layout={[[1]]} />
	</Cell>
	<Cell>
		<Skeleton layout={[[1]]} />
	</Cell>
{:else}
	<Row
		on:click={() => dispatch('select', { id: job.id, payloadData })}
		class={twMerge(
			selected === job.id ? 'bg-surface-selected' : 'hover:bg-surface-hover',
			'cursor-pointer rounded-md'
		)}
		on:hover={(e) => (hovering = e.detail ? true : false)}
	>
		<SchemaPickerRow {payloadData} date={job.started_at} {hovering}>
			<svelte:fragment slot="start">
				<div class="center-center">
					<div
						class={twMerge(
							'rounded-full w-2 h-2 animate-pulse',
							job.suspend ? 'bg-violet-400' : 'bg-orange-400'
						)}
						title="Running"
					></div>
				</div>
			</svelte:fragment>
			<svelte:fragment slot="extra">
				<div class="center-center {hovering ? '' : '!hidden'}">
					<a
						target="_blank"
						href="{base}/run/{job.id}?workspace={$workspaceStore}"
						class="text-right float-right text-secondary"
						title="See run detail in a new tab"
					>
						<ExternalLink size={16} />
					</a>
				</div>
			</svelte:fragment>
		</SchemaPickerRow>
	</Row>
{/if}
