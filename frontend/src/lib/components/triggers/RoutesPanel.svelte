<script lang="ts">
	import { Button } from '../common'
	import { userStore, workspaceStore } from '$lib/stores'
	import { HttpTriggerService } from '$lib/gen'
	import { RouteIcon } from 'lucide-svelte'
	import type { FlowEditorContext } from '$lib/components/flows/types'
	import { getContext } from 'svelte'

	import Skeleton from '../common/skeleton/Skeleton.svelte'
	import RouteEditor from './RouteEditor.svelte'
	import { canWrite } from '$lib/utils'
	import Alert from '../common/alert/Alert.svelte'

	export let isFlow: boolean
	export let path: string
	export let newFlow: boolean = false

	let routeEditor: RouteEditor

	$: path && loadTriggers()
	export async function loadTriggers() {
		try {
			$httpTriggers = (
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

	const { httpTriggers } = getContext<FlowEditorContext>('FlowEditorContext')
	$httpTriggers ?? loadTriggers()
</script>

<RouteEditor
	on:update={() => {
		loadTriggers()
	}}
	bind:this={routeEditor}
/>

<div class="flex flex-col gap-4">
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

	{#if $httpTriggers}
		{#if $httpTriggers.length == 0}
			<div class="text-xs text-secondary"> No http routes </div>
		{:else}
			<div class="flex flex-col divide-y pt-2">
				{#each $httpTriggers as $httpTriggers ($httpTriggers.path)}
					<div class="grid grid-cols-5 text-2xs items-center py-2">
						<div class="col-span-2 truncate">{$httpTriggers.path}</div>
						<div class="col-span-2 truncate">
							{$httpTriggers.http_method.toUpperCase()} /{$httpTriggers.route_path}
						</div>
						<div class="flex justify-end">
							<button
								on:click={() => routeEditor?.openEdit($httpTriggers.path, isFlow)}
								class="px-2"
							>
								{#if $httpTriggers.canWrite}
									Edit
								{:else}
									View
								{/if}
							</button>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	{:else}
		<Skeleton layout={[[8]]} />
	{/if}

	{#if newFlow}
		<Alert title="Triggers disabled" type="warning" size="xs">
			Deploy the flow to enable routes triggers.
		</Alert>
	{/if}
</div>
