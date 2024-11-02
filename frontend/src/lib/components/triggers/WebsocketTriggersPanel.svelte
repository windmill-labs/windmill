<script lang="ts">
	import { Button } from '../common'
	import { userStore, workspaceStore } from '$lib/stores'
	import { WebsocketTriggerService, type WebsocketTrigger } from '$lib/gen'
	import { UnplugIcon } from 'lucide-svelte'

	import Skeleton from '../common/skeleton/Skeleton.svelte'
	import { canWrite } from '$lib/utils'
	import Alert from '../common/alert/Alert.svelte'
	import type { TriggerContext } from '../triggers'
	import { getContext } from 'svelte'
	import WebsocketTriggerEditor from './WebsocketTriggerEditor.svelte'
	import { isCloudHosted } from '$lib/cloud'

	export let isFlow: boolean
	export let path: string
	export let newItem: boolean = false

	let wsTriggerEditor: WebsocketTriggerEditor

	$: path && loadTriggers()

	const { triggersCount } = getContext<TriggerContext>('TriggerContext')

	let wsTriggers: (WebsocketTrigger & { canWrite: boolean })[] | undefined = undefined
	export async function loadTriggers() {
		try {
			wsTriggers = (
				await WebsocketTriggerService.listWebsocketTriggers({
					workspace: $workspaceStore ?? '',
					path,
					isFlow
				})
			).map((x) => {
				return { canWrite: canWrite(x.path, x.extra_perms!, $userStore), ...x }
			})
			$triggersCount = { ...($triggersCount ?? {}), websocket_count: wsTriggers?.length }
		} catch (e) {
			console.error('impossible to load WS triggers', e)
		}
	}
</script>

<WebsocketTriggerEditor
	on:update={() => {
		loadTriggers()
	}}
	bind:this={wsTriggerEditor}
/>

<div class="flex flex-col gap-4">
	{#if !newItem}
		{#if isCloudHosted()}
			<Alert title="Not compatible with multi-tenant cloud" type="warning">
				Websocket triggers are disabled in the multi-tenant cloud.
			</Alert>
		{:else if $userStore?.is_admin || $userStore?.is_super_admin}
			<Button
				on:click={() => wsTriggerEditor?.openNew(isFlow, path)}
				variant="border"
				color="light"
				size="xs"
				startIcon={{ icon: UnplugIcon }}
			>
				New WS Trigger
			</Button>
		{:else}
			<Alert title="Only workspace admins can create routes" type="warning" size="xs" />
		{/if}
	{/if}

	{#if wsTriggers}
		{#if wsTriggers.length == 0}
			<div class="text-xs text-secondary"> No WS triggers </div>
		{:else}
			<div class="flex flex-col divide-y pt-2">
				{#each wsTriggers as wsTriggers (wsTriggers.path)}
					<div class="grid grid-cols-5 text-2xs items-center py-2">
						<div class="col-span-2 truncate">{wsTriggers.path}</div>
						<div class="col-span-2 truncate">
							{wsTriggers.url}
						</div>
						<div class="flex justify-end">
							<button
								on:click={() => wsTriggerEditor?.openEdit(wsTriggers.path, isFlow)}
								class="px-2"
							>
								{#if wsTriggers.canWrite}
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
			Deploy the {isFlow ? 'flow' : 'script'} to add WS triggers.
		</Alert>
	{/if}
</div>
