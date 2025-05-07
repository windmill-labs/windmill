<script lang="ts">
	import { SCRIPT_VIEW_SHOW_SCHEDULE_RUN_LATER } from '$lib/consts'
	import Label from './Label.svelte'
	import Toggle from './Toggle.svelte'
	import Tooltip from './Tooltip.svelte'
	import { userStore, workerTags, workspaceStore } from '$lib/stores'
	import { Button } from './common'
	import { WorkerService } from '$lib/gen'
	import DateTimeInput from './DateTimeInput.svelte'

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
	export let invisible_to_owner: boolean | undefined
	export let overrideTag: string | undefined
	loadWorkerGroups()

	async function loadWorkerGroups() {
		if (!$workerTags) {
			$workerTags = await WorkerService.getCustomTags({ workspace: $workspaceStore })
		}
	}
</script>

<div class="flex flex-col gap-4 p-4">
	<div class="flex flex-col gap-2">
		{#if SCRIPT_VIEW_SHOW_SCHEDULE_RUN_LATER}
			<div class="">
				<Label label="Schedule to run later" />

				<div class="flex flex-row items-center my-1">
					<div>
						<label for="run-time"></label>
						<DateTimeInput
							value={scheduledForStr}
							on:change={(e) => {
								console.log('e.detail', e.detail)
								scheduledForStr = e.detail
							}}
						/>
					</div>
					<Button
						variant="border"
						color="light"
						size="xs"
						btnClasses="mx-2 "
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
