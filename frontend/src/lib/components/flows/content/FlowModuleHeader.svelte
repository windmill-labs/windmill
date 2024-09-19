<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { type FlowModule } from '$lib/gen'
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
	import { getLatestHashForScript } from '$lib/scripts'
	import type { FlowBuilderWhitelabelCustomUi } from '$lib/components/custom_ui'
	import FlowModuleWorkerTagSelect from './FlowModuleWorkerTagSelect.svelte'

	export let module: FlowModule
	const { scriptEditorDrawer } = getContext<FlowEditorContext>('FlowEditorContext')

	const dispatch = createEventDispatcher()
	let customUi: undefined | FlowBuilderWhitelabelCustomUi = getContext('customUi')

	$: moduleRetry = module.retry?.constant || module.retry?.exponential
</script>

<div class="flex flex-row space-x-1">
	{#if module.value.type === 'script' || module.value.type === 'rawscript' || module.value.type == 'flow'}
		{#if moduleRetry}
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
				on:click={() => dispatch('toggleMock')}
			>
				<Voicemail size={14} />
				<svelte:fragment slot="text">Mock</svelte:fragment>
			</Popover>
		{/if}
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
		<FlowModuleWorkerTagSelect bind:tag={module.value.tag_override} />
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
	{/if}
	<div class="px-0.5" />
	{#if module.value.type === 'rawscript'}
		<FlowModuleWorkerTagSelect bind:tag={module.value.tag} />
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
