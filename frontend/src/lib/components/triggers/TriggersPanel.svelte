<script lang="ts">
	import { Button } from '../common'
	import { workspaceStore } from '$lib/stores'
	import { TriggerService, type Trigger } from '$lib/gen'
	import { RouteIcon } from 'lucide-svelte'

	import Skeleton from '../common/skeleton/Skeleton.svelte'
	import TriggerEditor from './TriggerEditor.svelte'

	export let isFlow: boolean
	export let path: string

	let triggerEditor: TriggerEditor

	let triggers: Trigger[] | undefined = undefined

	$: path && loadTriggers()
	async function loadTriggers() {
		try {
			triggers = await TriggerService.listTriggers({
				workspace: $workspaceStore ?? '',
				path,
				isFlow
			})
		} catch (e) {
			console.error('impossible to load triggers')
		}
	}
</script>

<TriggerEditor
	on:update={() => {
		loadTriggers()
	}}
	bind:this={triggerEditor}
/>

<div class="p-2 flex flex-col">
	<Button
		on:click={() => triggerEditor?.openNew(isFlow, path)}
		variant="border"
		color="light"
		size="xs"
		startIcon={{ icon: RouteIcon }}
	>
		New Trigger
	</Button>
</div>

{#if triggers}
	{#if triggers.length == 0}
		<div class="text-xs text-secondary px-2"> No triggers </div>
	{:else}
		<div class="flex flex-col divide-y px-2 pt-2">
			{#each triggers as trigger (trigger.path)}
				<div class="grid grid-cols-6 text-2xs items-center py-2">
					<div class="col-span-3 truncate">{trigger.path}</div>
					<div>route: /{trigger.route_path}</div>
					<button on:click={() => triggerEditor?.openEdit(trigger.path, isFlow)}>Edit</button>
				</div>
			{/each}
		</div>
	{/if}
{:else}
	<Skeleton layout={[[8]]} />
{/if}
