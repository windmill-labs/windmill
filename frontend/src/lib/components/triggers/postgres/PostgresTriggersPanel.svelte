<script lang="ts">
	import { userStore, workspaceStore } from '$lib/stores'
	import { PostgresTriggerService, type PostgresTrigger } from '$lib/gen'
	import { UnplugIcon } from 'lucide-svelte'

	import { canWrite } from '$lib/utils'
	import { getContext } from 'svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { Alert, Button, Skeleton } from '$lib/components/common'
	import type { TriggerContext } from '$lib/components/triggers'
	import PostgresTriggerEditor from './PostgresTriggerEditor.svelte'

	export let isFlow: boolean
	export let path: string
	export let newItem: boolean = false

	let postgresTriggerEditor: PostgresTriggerEditor

	$: path && loadTriggers()

	const { triggersCount } = getContext<TriggerContext>('TriggerContext')

	let databaseTriggers: (PostgresTrigger & { canWrite: boolean })[] | undefined = undefined
	export async function loadTriggers() {
		try {
			databaseTriggers = (
				await PostgresTriggerService.listPostgresTriggers({
					workspace: $workspaceStore ?? '',
					path,
					isFlow
				})
			).map((x) => {
				return { canWrite: canWrite(x.path, x.extra_perms!, $userStore), ...x }
			})
			$triggersCount = { ...($triggersCount ?? {}), postgres_count: databaseTriggers?.length }
		} catch (e) {
			console.error('impossible to load Postgres triggers', e)
		}
	}
</script>

<PostgresTriggerEditor
	on:update={() => {
		loadTriggers()
	}}
	bind:this={postgresTriggerEditor}
/>

<div class="flex flex-col gap-4">
	{#if !newItem}
		{#if isCloudHosted()}
			<Alert title="Not compatible with multi-tenant cloud" type="warning">
				Postgres triggers are disabled in the multi-tenant cloud.
			</Alert>
		{:else if $userStore?.is_admin || $userStore?.is_super_admin}
			<Button
				on:click={() => postgresTriggerEditor?.openNew(isFlow, path)}
				variant="border"
				color="light"
				size="xs"
				startIcon={{ icon: UnplugIcon }}
			>
				New Postgres trigger
			</Button>
		{:else}
			<Alert title="Only workspace admins can create routes" type="warning" size="xs" />
		{/if}
	{/if}

	{#if databaseTriggers}
		{#if databaseTriggers.length == 0}
			<div class="text-xs text-secondary"> No Postgres triggers </div>
		{:else}
			<div class="flex flex-col divide-y pt-2">
				{#each databaseTriggers as databaseTriggers (databaseTriggers.path)}
					<div class="grid grid-cols-5 text-2xs items-center py-2">
						<div class="col-span-2 truncate">{databaseTriggers.path}</div>
						<div class="flex justify-end">
							<button
								on:click={() => postgresTriggerEditor?.openEdit(databaseTriggers.path, isFlow)}
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
			Deploy the {isFlow ? 'flow' : 'script'} to add Postgres triggers.
		</Alert>
	{/if}
</div>
