<script lang="ts">
	import { Button } from '../common'
	import { workspaceStore } from '$lib/stores'
	import { TriggerService, type Trigger } from '$lib/gen'
	import { RouteIcon } from 'lucide-svelte'

	import Skeleton from '../common/skeleton/Skeleton.svelte'
	import RouteEditor from './RouteEditor.svelte'

	export let isFlow: boolean
	export let path: string

	let routeEditor: RouteEditor

	let triggers: Trigger[] | undefined = undefined

	$: path && loadTriggers()
	async function loadTriggers() {
		try {
			triggers = await TriggerService.listTriggers({
				workspace: $workspaceStore ?? '',
				path,
				isFlow,
				kind: 'http'
			})
		} catch (e) {
			console.error('impossible to load http routes')
		}
	}
</script>

<RouteEditor
	on:update={() => {
		loadTriggers()
	}}
	bind:this={routeEditor}
/>

<div class="p-2 flex flex-col">
	<Button
		on:click={() => routeEditor?.openNew(isFlow, path)}
		variant="border"
		color="light"
		size="xs"
		startIcon={{ icon: RouteIcon }}
	>
		New Route
	</Button>
</div>

{#if triggers}
	{#if triggers.length == 0}
		<div class="text-xs text-secondary px-2"> No http routes </div>
	{:else}
		<div class="flex flex-col divide-y px-2 pt-2">
			{#each triggers as trigger (trigger.path)}
				<div class="grid grid-cols-5 text-2xs items-center py-2">
					<div class="col-span-2 truncate">{trigger.path}</div>
					<div class="col-span-2 truncate">endpoint: /{trigger.route_path}</div>
					<button on:click={() => routeEditor?.openEdit(trigger.path, isFlow)}>Edit</button>
				</div>
			{/each}
		</div>
	{/if}
{:else}
	<Skeleton layout={[[8]]} />
{/if}
