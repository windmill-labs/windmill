<script lang="ts">
	import { Button } from '../common'
	import { userStore, workspaceStore } from '$lib/stores'
	import { HttpTriggerService, type HttpTrigger } from '$lib/gen'
	import { RouteIcon } from 'lucide-svelte'

	import Skeleton from '../common/skeleton/Skeleton.svelte'
	import RouteEditor from './RouteEditor.svelte'
	import { canWrite } from '$lib/utils'

	export let isFlow: boolean
	export let path: string

	let routeEditor: RouteEditor

	export let triggers: (HttpTrigger & { canWrite: boolean })[] | undefined = undefined

	$: path && loadTriggers()
	export async function loadTriggers() {
		try {
			triggers = (
				await HttpTriggerService.listHttpTriggers({
					workspace: $workspaceStore ?? '',
					path,
					isFlow
				})
			).map((x) => {
				return { canWrite: canWrite(x.path, x.extra_perms!, $userStore), ...x }
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
	{#if $userStore?.is_admin || $userStore?.is_super_admin}
		<Button
			on:click={() => routeEditor?.openNew(isFlow, path)}
			variant="border"
			color="light"
			size="xs"
			startIcon={{ icon: RouteIcon }}
		>
			New Route
		</Button>
	{/if}
</div>

{#if triggers}
	{#if triggers.length == 0}
		<div class="text-xs text-secondary px-2"> No http routes </div>
	{:else}
		<div class="flex flex-col divide-y px-2 pt-2">
			{#each triggers as trigger (trigger.path)}
				<div class="grid grid-cols-5 text-2xs items-center py-2">
					<div class="col-span-2 truncate">{trigger.path}</div>
					<div class="col-span-2 truncate">
						{trigger.http_method.toUpperCase()} /{trigger.route_path}
					</div>
					<button on:click={() => routeEditor?.openEdit(trigger.path, isFlow)}>
						{#if trigger.canWrite}
							Edit
						{:else}
							View
						{/if}
					</button>
				</div>
			{/each}
		</div>
	{/if}
{:else}
	<Skeleton layout={[[8]]} />
{/if}
