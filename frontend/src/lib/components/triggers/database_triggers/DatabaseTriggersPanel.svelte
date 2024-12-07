<script lang="ts">
	import { userStore, workspaceStore } from '$lib/stores'
	import { WebsocketTriggerService, type WebsocketTrigger } from '$lib/gen'
	import { UnplugIcon } from 'lucide-svelte'

	import { canWrite } from '$lib/utils'
	import { getContext } from 'svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { Alert, Button, Skeleton } from '$lib/components/common'
	import type { TriggerContext } from '$lib/components/triggers'
	import DatabaseTriggerEditor from './DatabaseTriggerEditor.svelte'

	export let isFlow: boolean
	export let path: string
	export let newItem: boolean = false

	let databaseTriggerEditor: DatabaseTriggerEditor

	$: path && loadTriggers()

	const { triggersCount } = getContext<TriggerContext>('TriggerContext')

	let databaseTriggers: (WebsocketTrigger & { canWrite: boolean })[] | undefined = undefined
	export async function loadTriggers() {
		try {
			databaseTriggers = (
				await WebsocketTriggerService.listWebsocketTriggers({
					workspace: $workspaceStore ?? '',
					path,
					isFlow
				})
			).map((x) => {
				return { canWrite: canWrite(x.path, x.extra_perms!, $userStore), ...x }
			})
			$triggersCount = { ...($triggersCount ?? {}), database_count: databaseTriggers?.length }
		} catch (e) {
			console.error('impossible to load Database triggers', e)
		}
	}
</script>

<DatabaseTriggerEditor
	on:update={() => {
		loadTriggers()
	}}
	bind:this={databaseTriggerEditor}
/>

<div class="flex flex-col gap-4">
	{#if !newItem}
		{#if isCloudHosted()}
			<Alert title="Not compatible with multi-tenant cloud" type="warning">
				Database triggers are disabled in the multi-tenant cloud.
			</Alert>
		{:else if $userStore?.is_admin || $userStore?.is_super_admin}
			<Button
				on:click={() => databaseTriggerEditor?.openNew(isFlow, path)}
				variant="border"
				color="light"
				size="xs"
				startIcon={{ icon: UnplugIcon }}
			>
				New Database Trigger
			</Button>
		{:else}
			<Alert title="Only workspace admins can create routes" type="warning" size="xs" />
		{/if}
	{/if}

	{#if databaseTriggers}
		{#if databaseTriggers.length == 0}
			<div class="text-xs text-secondary"> No Database triggers </div>
		{:else}
			<div class="flex flex-col divide-y pt-2">
				{#each databaseTriggers as databaseTriggers (databaseTriggers.path)}
					<div class="grid grid-cols-5 text-2xs items-center py-2">
						<div class="col-span-2 truncate">{databaseTriggers.path}</div>
						<div class="col-span-2 truncate">
							{databaseTriggers.url}
						</div>
						<div class="flex justify-end">
							<button
								on:click={() => databaseTriggerEditor?.openEdit(databaseTriggers.path, isFlow)}
								class="px-2"
							>
								{#if databaseTriggers.canWrite}
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
			Deploy the {isFlow ? 'flow' : 'script'} to add Database triggers.
		</Alert>
	{/if}
</div>
