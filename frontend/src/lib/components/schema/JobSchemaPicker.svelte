<script lang="ts">
	import SchemaPicker from '$lib/components/schema/SchemaPicker.svelte'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import { InputService } from '$lib/gen/index.js'
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import { ExternalLink } from 'lucide-svelte'
	import { base } from '$lib/base'

	export let job: any
	export let selected = false
	export let payloadData: any | undefined = undefined
	const dispatch = createEventDispatcher()

	loadLargeArgs(job.id, undefined, false)

	async function loadLargeArgs(
		id: string | undefined,
		input: boolean | undefined,
		allowLarge: boolean
	): Promise<any> {
		if (!id) return
		payloadData = await InputService.getArgsFromHistoryOrSavedInput({
			jobOrInputId: id,
			workspace: $workspaceStore!,
			input,
			allowLarge
		})
		loadingArgs = false
	}

	let loadingArgs = true
</script>

<div>
	{#if loadingArgs}
		<Skeleton layout={[[8]]} />
	{:else}
		<SchemaPicker
			{payloadData}
			on:updateSchema
			on:applyArgs={async () => {
				dispatch('selected_args', payloadData)
			}}
			on:select={() => dispatch('select', { jobId: job.id, payloadData })}
			date={job.created_at}
			{selected}
		>
			<svelte:fragment slot="start">
				<div class="center-center">
					<div class="rounded-full w-2 h-2 {job.success ? 'bg-green-400' : 'bg-red-400'}" />
				</div>
			</svelte:fragment>
			<svelte:fragment slot="extra">
				<div class="center-center">
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
		</SchemaPicker>
	{/if}
</div>
