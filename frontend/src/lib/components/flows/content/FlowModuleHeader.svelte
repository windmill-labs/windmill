<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { type FlowModule } from '$lib/gen'
	import { createEventDispatcher, getContext } from 'svelte'
	import {
		Bed,
		Database,
		ExternalLink,
		Gauge,
		GitFork,
		Pen,
		PhoneIncoming,
		RefreshCcw,
		Repeat,
		Save,
		Square,
		Pin
	} from 'lucide-svelte'
	import Popover from '../../Popover.svelte'
	import type { FlowEditorContext } from '../types'
	import { sendUserToast } from '$lib/utils'
	import { getLatestHashForScript } from '$lib/scripts'
	import type { FlowBuilderWhitelabelCustomUi } from '$lib/components/custom_ui'
	import FlowModuleWorkerTagSelect from './FlowModuleWorkerTagSelect.svelte'

	export let module: FlowModule
	export let tag: string | undefined
	const { scriptEditorDrawer } = getContext<FlowEditorContext>('FlowEditorContext')

	const dispatch = createEventDispatcher()
	let customUi: undefined | FlowBuilderWhitelabelCustomUi = getContext('customUi')
</script>

<div class="flex flex-row space-x-1">
	{#if module.value.type === 'script' || module.value.type === 'rawscript' || module.value.type == 'flow'}
		{#if module.retry?.constant || module.retry?.exponential}
			<Popover
				placement="bottom"
				class="center-center rounded p-2 bg-blue-100 text-blue-800 border border-blue-300 hover:bg-blue-200 dark:bg-frost-700 dark:text-frost-100 dark:border-frost-600"
				on:click={() => dispatch('toggleRetry')}
			>
				<Repeat size={14} />
				<svelte:fragment slot="text">Retries</svelte:fragment>
			</Popover>
		{/if}
		{#if module?.value?.['concurrent_limit'] != undefined}
			<Popover
				placement="bottom"
				class="center-center rounded p-2 bg-blue-100 text-blue-800 border border-blue-300 hover:bg-blue-200 dark:bg-frost-700 dark:text-frost-100 dark:border-frost-600"
				on:click={() => dispatch('toggleConcurrency')}
			>
				<Gauge size={14} />
				<svelte:fragment slot="text">Concurrency Limits</svelte:fragment>
			</Popover>
		{/if}
		{#if module.cache_ttl != undefined}
			<Popover
				placement="bottom"
				class="center-center rounded p-2 bg-blue-100 text-blue-800 border border-blue-300 hover:bg-blue-200 dark:bg-frost-700 dark:text-frost-100 dark:border-frost-600"
				on:click={() => dispatch('toggleCache')}
			>
				<Database size={14} />
				<svelte:fragment slot="text">Cache</svelte:fragment>
			</Popover>
		{/if}
		{#if module.stop_after_if || module.stop_after_all_iters_if}
			<Popover
				placement="bottom"
				class="center-center rounded p-2 bg-blue-100 text-blue-800 border border-blue-300 hover:bg-blue-200 dark:bg-frost-700 dark:text-frost-100 dark:border-frost-600"
				on:click={() => dispatch('toggleStopAfterIf')}
			>
				<Square size={14} />
				<svelte:fragment slot="text">Early stop/break</svelte:fragment>
			</Popover>
		{/if}
		{#if module.suspend}
			<Popover
				placement="bottom"
				class="center-center rounded p-2 bg-blue-100 text-blue-800 border border-blue-300 hover:bg-blue-200 dark:bg-frost-700 dark:text-frost-100 dark:border-frost-600"
				on:click={() => dispatch('toggleSuspend')}
			>
				<PhoneIncoming size={14} />
				<svelte:fragment slot="text">Suspend</svelte:fragment>
			</Popover>
		{/if}
		{#if module.sleep}
			<Popover
				placement="bottom"
				class="center-center rounded p-2 bg-blue-100 text-blue-800 border border-blue-300 hover:bg-blue-200 dark:bg-frost-700 dark:text-frost-100 dark:border-frost-600"
				on:click={() => dispatch('toggleSleep')}
			>
				<Bed size={14} />
				<svelte:fragment slot="text">Sleep</svelte:fragment>
			</Popover>
		{/if}
		{#if module.mock?.enabled}
			<Popover
				placement="bottom"
				class="center-center rounded p-2 bg-blue-100 text-blue-800 border border-blue-300 hover:bg-blue-200 dark:bg-frost-700 dark:text-frost-100 dark:border-frost-600"
				on:click={() => dispatch('togglePin')}
			>
				<Pin size={14} />
				<svelte:fragment slot="text">This step is pinned</svelte:fragment>
			</Popover>
		{/if}
	{/if}
	{#if module.value.type === 'script'}
		<div class="w-2"></div>

		{#if !module.value.path.startsWith('hub/') && customUi?.scriptEdit != false}
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
				tooltip={module.value.hash != undefined
					? 'Script is locked with a specific hash'
					: undefined}
				disabled={module.value.hash != undefined}
			>
				Edit
			</Button>
		{/if}
		{#if customUi?.tagEdit != false}
			<FlowModuleWorkerTagSelect
				nullTag={tag}
				tag={module.value.tag_override}
				on:change={(e) => dispatch('tagChange', e.detail)}
			/>
		{/if}
		{#if customUi?.scriptFork != false}
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
	{:else if module.value.type === 'flow'}
		<Button
			size="xs"
			color="light"
			on:click={async () => {
				if (module.value.type == 'flow') {
					window.open(`/flows/edit/${module.value.path}`, '_blank', 'noopener,noreferrer')
				}
			}}
			startIcon={{ icon: Pen }}
			iconOnly={false}
		>
			Edit <ExternalLink size={12} />
		</Button>
		<Button
			size="xs"
			color="light"
			on:click={async () => {
				dispatch('reload')
			}}
			startIcon={{
				icon: RefreshCcw
			}}
			iconOnly={true}
		/>
	{/if}
	<div class="px-0.5"></div>
	{#if module.value.type === 'rawscript'}
		<FlowModuleWorkerTagSelect
			tag={module.value.tag}
			on:change={(e) => dispatch('tagChange', e.detail)}
		/>
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
