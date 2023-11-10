<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { WorkerService, type FlowModule } from '$lib/gen'
	import { createEventDispatcher, getContext } from 'svelte'
	import {
		Bed,
		Database,
		Gauge,
		GitFork,
		Pen,
		PhoneIncoming,
		Repeat,
		Save,
		Square,
		Voicemail
	} from 'lucide-svelte'
	import Popover from '../../Popover.svelte'
	import type { FlowEditorContext } from '../types'
	import { sendUserToast } from '$lib/utils'
	import { workerTags } from '$lib/stores'
	import { getLatestHashForScript } from '$lib/scripts'

	export let module: FlowModule
	const { scriptEditorDrawer, flowStore, selectedId } =
		getContext<FlowEditorContext>('FlowEditorContext')

	const dispatch = createEventDispatcher()

	loadWorkerGroups()

	async function loadWorkerGroups() {
		if (!$workerTags) {
			$workerTags = await WorkerService.getCustomTags()
		}
	}

	$: moduleRetry = module.retry?.constant || module.retry?.exponential
</script>

<div class="flex flex-row space-x-2">
	{#if module.value.type === 'script' || module.value.type === 'rawscript' || module.value.type == 'flow'}
		<Popover
			placement="bottom"
			class="center-center rounded p-2 
			{moduleRetry
				? 'bg-blue-100 text-blue-800 border border-blue-300 hover:bg-blue-200 dark:bg-frost-700 dark:text-frost-100 dark:border-frost-600'
				: 'bg-surface text-primay hover:bg-hover'}"
			on:click={() => dispatch('toggleRetry')}
		>
			<Repeat size={14} />
			<svelte:fragment slot="text">Retries</svelte:fragment>
		</Popover>
		<Popover
			placement="bottom"
			class="center-center rounded p-2 
		{module?.value?.['concurrency_limit'] != undefined
				? 'bg-blue-100 text-blue-800 border border-blue-300 hover:bg-blue-200 dark:bg-frost-700 dark:text-frost-100 dark:border-frost-600'
				: 'bg-surface text-primay hover:bg-hover'}"
			on:click={() => dispatch('toggleConcurrency')}
		>
			<Gauge size={14} />
			<svelte:fragment slot="text">Concurrency Limits</svelte:fragment>
		</Popover>
		<Popover
			placement="bottom"
			class="center-center rounded p-2 
		{module.cache_ttl != undefined
				? 'bg-blue-100 text-blue-800 border border-blue-300 hover:bg-blue-200 dark:bg-frost-700 dark:text-frost-100 dark:border-frost-600'
				: 'bg-surface text-primay hover:bg-hover'}"
			on:click={() => dispatch('toggleCache')}
		>
			<Database size={14} />
			<svelte:fragment slot="text">Cache</svelte:fragment>
		</Popover>
		<Popover
			placement="bottom"
			class="center-center rounded p-2
			{module.stop_after_if
				? 'bg-blue-100 text-blue-800 border border-blue-300 hover:bg-blue-200 dark:bg-frost-700 dark:text-frost-100 dark:border-frost-600'
				: 'bg-surface text-primay hover:bg-hover'}"
			on:click={() => dispatch('toggleStopAfterIf')}
		>
			<Square size={14} />
			<svelte:fragment slot="text">Early stop/break</svelte:fragment>
		</Popover>
		<Popover
			placement="bottom"
			class="center-center rounded p-2 
			{module.suspend
				? 'bg-blue-100 text-blue-800 border border-blue-300 hover:bg-blue-200 dark:bg-frost-700 dark:text-frost-100 dark:border-frost-600'
				: 'bg-surface text-primay hover:bg-hover'}"
			on:click={() => dispatch('toggleSuspend')}
		>
			<PhoneIncoming size={14} />
			<svelte:fragment slot="text">Suspend</svelte:fragment>
		</Popover>
		<Popover
			placement="bottom"
			class="center-center rounded p-2
			{module.sleep
				? 'bg-blue-100 text-blue-800 border border-blue-300 hover:bg-blue-200 dark:bg-frost-700 dark:text-frost-100 dark:border-frost-600'
				: 'bg-surface text-primay hover:bg-hover'}"
			on:click={() => dispatch('toggleSleep')}
		>
			<Bed size={14} />
			<svelte:fragment slot="text">Sleep</svelte:fragment>
		</Popover>
		<Popover
			placement="bottom"
			class="center-center rounded p-2
		{module.mock?.enabled
				? 'bg-blue-100  text-blue-800 border border-blue-300 hover:bg-blue-200'
				: 'bg-surface text-primay hover:bg-hover'}"
			on:click={() => dispatch('toggleMock')}
		>
			<Voicemail size={14} />
			<svelte:fragment slot="text">Mock</svelte:fragment>
		</Popover>
	{/if}
	{#if module.value.type === 'script'}
		<div class="w-2" />
		{#if !module.value.path.startsWith('hub/')}
			<Button
				size="xs"
				color="light"
				on:click={async () => {
					if (module.value.type == 'script') {
						const hash = module.value.hash ?? (await getLatestHashForScript(module.value.path))
						$scriptEditorDrawer?.openDrawer(hash, () => {
							dispatch('reload')
							sendUserToast('Script has been updated')
						})
					}
				}}
				startIcon={{ icon: Pen }}
				iconOnly={false}
				disabled={module.value.hash != undefined}
			>
				Edit {#if module.value.hash != undefined} (locked hash){/if}
			</Button>
		{/if}
		<Button
			size="xs"
			color="light"
			on:click={() => dispatch('fork')}
			startIcon={{ icon: GitFork }}
			iconOnly={false}
		>
			Fork
		</Button>
	{/if}

	{#if module.value.type === 'rawscript'}
		{#if $workerTags}
			{#if $workerTags?.length > 0}
				<div class="w-40">
					{#if $flowStore.tag == undefined}
						<select
							placeholder="Worker group"
							bind:value={module.value.tag}
							on:change={(e) => {
								if (module.value.type === 'rawscript') {
									if (module.value.tag == '') {
										module.value.tag = undefined
									}
								}
							}}
						>
							{#if module.value.tag}
								<option value="">reset to default</option>
							{:else}
								<option value="" disabled selected>Worker Group</option>
							{/if}
							{#each $workerTags ?? [] as tag (tag)}
								<option value={tag}>{tag}</option>
							{/each}
						</select>
					{:else}
						<button
							title="Worker Group is defined at the flow level"
							class="w-full text-left items-center font-normal p-1 border text-xs rounded"
							on:click={() => ($selectedId = 'settings-worker-group')}
						>
							Flow's WG: {$flowStore.tag}
						</button>
					{/if}
				</div>
			{/if}
		{/if}
		<Button
			size="xs"
			color="light"
			startIcon={{ icon: Save }}
			on:click={() => dispatch('createScriptFromInlineScript')}
			iconOnly={false}
		>
			Save to workspace
		</Button>
	{/if}
</div>
