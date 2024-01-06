<script lang="ts">
	import { SCRIPT_VIEW_SHOW_SCHEDULE_RUN_LATER } from '$lib/consts'
	import Label from './Label.svelte'
	import CloseButton from './common/CloseButton.svelte'
	import Toggle from './Toggle.svelte'
	import Tooltip from './Tooltip.svelte'
	import { userStore, workerTags } from '$lib/stores'
	import { Button } from './common'
	import { getToday } from '$lib/utils'
	import { WorkerService } from '$lib/gen'

	export let runnable:
		| {
				summary?: string
				description?: string
				path?: string
				is_template?: boolean
				hash?: string
				kind?: string
				can_write?: boolean
				created_at?: string
				created_by?: string
				extra_perms?: Record<string, boolean>
		  }
		| undefined

	export let scheduledForStr: string | undefined
	export let invisible_to_owner: false | undefined
	export let overrideTag: string | undefined

	loadWorkerGroups()

	async function loadWorkerGroups() {
		if (!$workerTags) {
			$workerTags = await WorkerService.getCustomTags()
		}
	}
</script>

<div class="flex flex-col gap-4 p-2">
	<div class="flex flex-col gap-2">
		{#if SCRIPT_VIEW_SHOW_SCHEDULE_RUN_LATER}
			<div class="">
				<Label label="Schedule to run later">
					<svelte:fragment slot="action">
						<CloseButton on:close noBg />
					</svelte:fragment>
				</Label>

				<div class="flex flex-row items-end">
					<div class="w-max md:w-2/3 mt-2 mb-1">
						<label for="run-time" />
						<input
							class="inline-block"
							type="datetime-local"
							id="run-time"
							name="run-scheduled-time"
							bind:value={scheduledForStr}
							min={getToday().toISOString().slice(0, 16)}
						/>
					</div>
					<Button
						variant="border"
						color="light"
						size="sm"
						btnClasses="mx-2 mb-1"
						on:click={() => {
							scheduledForStr = undefined
						}}
					>
						Clear
					</Button>
				</div>
			</div>
		{/if}
	</div>
	{#if !$userStore?.operator}
		{#if $workerTags && $workerTags?.length > 0}
			<div class="w-full">
				<select placeholder="Worker group" bind:value={overrideTag}>
					{#if overrideTag}
						<option value="">reset to default</option>
					{:else}
						<option value="" disabled selected>Override Worker Group Tag</option>
					{/if}
					{#each $workerTags ?? [] as tag (tag)}
						<option value={tag}>{tag}</option>
					{/each}
				</select>
			</div>
		{/if}
	{/if}

	{#if runnable?.path?.startsWith(`u/${$userStore?.username}`) != true && (runnable?.path?.split('/')?.length ?? 0) > 2}
		<div class="flex items-center gap-1">
			<Toggle
				options={{
					right: `make run invisible to others`
				}}
				bind:checked={invisible_to_owner}
			/>
			<Tooltip
				>By default, runs are visible to the owner(s) of the script or flow being triggered</Tooltip
			>
		</div>
	{/if}
</div>
