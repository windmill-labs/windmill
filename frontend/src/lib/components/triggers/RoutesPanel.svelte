<script lang="ts">
	import { Button } from '../common'
	import { userStore, workspaceStore } from '$lib/stores'
	import { HttpTriggerService, type HttpTrigger } from '$lib/gen'
	import { RouteIcon } from 'lucide-svelte'

	import Skeleton from '../common/skeleton/Skeleton.svelte'
	import RouteEditor from './RouteEditor.svelte'
	import { canWrite } from '$lib/utils'
	import Alert from '../common/alert/Alert.svelte'
	import type { TriggerContext } from '../triggers'
	import { getContext } from 'svelte'

	export let isFlow: boolean
	export let path: string
	export let newItem: boolean = false

	let routeEditor: RouteEditor

	$: path && loadTriggers()

	const { triggersCount } = getContext<TriggerContext>('TriggerContext')

	let httpTriggers: (HttpTrigger & { canWrite: boolean })[] | undefined = undefined
	export async function loadTriggers() {
		try {
			httpTriggers = (
				await HttpTriggerService.listHttpTriggers({
					workspace: $workspaceStore ?? '',
					path,
					isFlow
				})
			).map((x) => {
				return { canWrite: canWrite(x.path, x.extra_perms!, $userStore), ...x }
			})
			$triggersCount = { ...($triggersCount ?? {}), http_routes_count: httpTriggers?.length }
		} catch (e) {
			console.error('impossible to load http routes', e)
		}
	}
</script>

<RouteEditor
	on:update={() => {
		loadTriggers()
	}}
	bind:this={routeEditor}
/>

<div class="flex flex-col gap-4">
	{#if !newItem}
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
		{:else}
			<Alert title="Only workspace admins can create routes" type="warning" size="xs" />
		{/if}
	{/if}

	{#if httpTriggers}
		{#if httpTriggers.length == 0}
			<div class="text-xs text-secondary"> No http routes </div>
		{:else}
			<div class="flex flex-col divide-y pt-2">
				{#each httpTriggers as httpTriggers (httpTriggers.path)}
					<div class="grid grid-cols-5 text-2xs items-center py-2">
						<div class="col-span-2 truncate">{httpTriggers.path}</div>
						<div class="col-span-2 truncate">
							{httpTriggers.http_method.toUpperCase()} /{httpTriggers.route_path}
						</div>
						<div class="flex justify-end">
							<button
								on:click={() => routeEditor?.openEdit(httpTriggers.path, isFlow)}
								class="px-2"
							>
								{#if httpTriggers.canWrite}
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

	{#if newItem}
		<Alert title="Triggers disabled" type="warning" size="xs">
			Deploy the {isFlow ? 'flow' : 'script'} to add http routes.
		</Alert>
	{/if}
</div>
