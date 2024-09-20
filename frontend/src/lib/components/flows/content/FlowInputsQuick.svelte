<script lang="ts">
	import { isCloudHosted } from '$lib/cloud'
	import { sendUserToast } from '$lib/toast'
	import FlowScriptPickerQuick from '../pickers/FlowScriptPickerQuick.svelte'
	import WorkspaceScriptPickerQuick from '../pickers/WorkspaceScriptPickerQuick.svelte'
	import { defaultScriptLanguages, processLangs } from '$lib/scripts'
	import { defaultScripts } from '$lib/stores'
	import type { SupportedLanguage } from '$lib/common'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { FlowBuilderWhitelabelCustomUi } from '$lib/components/custom_ui'
	import PickHubScriptQuick from '../pickers/PickHubScriptQuick.svelte'
	import { type Script, type FlowModule } from '$lib/gen'
	import ListFiltersQuick from '$lib/components/home/ListFiltersQuick.svelte'
	import { APP_TO_ICON_COMPONENT } from '../../icons'
	import { Folder, Wand2 } from 'lucide-svelte'
	import { defaultIfEmptyString } from '$lib/utils'
	import type { FlowCopilotContext, FlowCopilotModule } from '../../copilot/flow'
	import type { FlowEditorContext } from '../../flows/types'
	import { copilotInfo } from '$lib/stores'
	import { nextId } from '../../flows/flowModuleNextId'
	import { twMerge } from 'tailwind-merge'
	import { fade } from 'svelte/transition'
	import { flip } from 'svelte/animate'
	import FlowInputsFlowQuick from '../content/FlowInputsFlowQuick.svelte'

	const dispatch = createEventDispatcher()

	export let failureModule: boolean
	export let summary: string | undefined = undefined
	export let filter = ''
	export let disableAi = false
	export let syncQuery = false
	export let preFilter: 'all' | 'workspace' | 'hub' = 'hub'
	export let funcDesc: string
	export let filteredItems: (Script & { marked?: string })[] = []
	export let hubCompletions: FlowCopilotModule['hubCompletions'] = []
	export let index: number
	export let modules: FlowModule[]
	export let owners: string[] = []
	export let loading = false

	let trigger = false
	let lang: FlowCopilotModule['lang'] = undefined
	let selectedCompletion: FlowCopilotModule['selectedCompletion'] = undefined

	const { flowStore, flowStateStore } = getContext<FlowEditorContext>('FlowEditorContext')
	const { modulesStore: copilotModulesStore, genFlow } =
		getContext<FlowCopilotContext>('FlowCopilotContext')

	export let selectedKind: 'action' | 'flow' | 'approval' | 'trigger' = 'action'

	let selected:
		| { kind: 'inline' | 'owner' | 'integrations'; name: string | undefined }
		| undefined = undefined

	let apps: string[] = []

	let customUi: undefined | FlowBuilderWhitelabelCustomUi = getContext('customUi')

	let kind: 'script' | 'failure' | 'approval' | 'trigger' = failureModule
		? 'failure'
		: summary == 'Trigger'
		? 'trigger'
		: summary == 'Approval'
		? 'approval'
		: 'script'

	$: if (selectedKind === 'trigger') {
		kind = 'trigger'
	} else if (selectedKind === 'approval') {
		kind = 'approval'
	} else if (selectedKind === 'action') {
		kind = 'script'
	}

	$: langs = processLangs(undefined, $defaultScripts?.order ?? Object.keys(defaultScriptLanguages))
		.map((l) => [defaultScriptLanguages[l], l])
		.filter(
			(x) => $defaultScripts?.hidden == undefined || !$defaultScripts.hidden.includes(x[1])
		) as [string, SupportedLanguage | 'docker'][]

	function displayLang(lang: SupportedLanguage | 'docker', kind: string) {
		if (lang == 'bun' || lang == 'python3' || lang == 'deno') {
			return true
		}
		if (lang == 'go') {
			return kind == 'script' || kind == 'trigger' || failureModule
		}

		if (lang == 'bash' || lang == 'nativets') {
			return kind == 'script'
		}
		return kind == 'script' && !failureModule
	}

	async function onGenerate() {
		if (!selectedCompletion && !$copilotInfo.exists_openai_resource_path) {
			sendUserToast(
				'Windmill AI is not enabled, you can activate it in the workspace settings',
				true
			)
			return
		}
		$copilotModulesStore = [
			{
				id: nextId($flowStateStore, $flowStore),
				type: trigger ? 'trigger' : 'script',
				description: funcDesc,
				code: '',
				source: selectedCompletion ? 'hub' : 'custom',
				hubCompletions,
				selectedCompletion,
				editor: undefined,
				lang
			}
		]
		genFlow?.(index, modules, true)
	}
</script>

<div class="flex flex-row divide-x w-full">
	<div class="w-44 flex-none h-full flex flex-col overflow-y-auto">
		{#if ['action', 'trigger', 'approval'].includes(selectedKind)}
			<!-- {#if funcDesc.length === 0 && preFilter == 'all'}
				<div class="font-mono text-xs text-secondary">
					<button
						class={twMerge(
							'w-full text-left py-2 px-3 hover:bg-surface-hover transition-all whitespace-nowrap flex flex-row gap-2 items-center',
							selected?.kind === 'inline' ? 'bg-surface-hover' : ''
						)}
						on:click={() => {
							selected =
								selected && selected.kind === 'inline'
									? undefined
									: { kind: 'inline', name: undefined }
						}}
						role="menuitem"
						tabindex="-1"
					>
						<Code size={14} />
						Inline Script
					</button>
				</div>
			{/if} -->
			{#if preFilter === 'all' || preFilter === 'workspace'}
				{#if preFilter !== 'workspace'}
					<div class="pt-2 pb-0 text-2xs text-secondary ml-2">Workspace Folders</div>
				{/if}
				{#if owners.length > 0}
					{#each owners as owner (owner)}
						<div in:fade={{ duration: 50 }} animate:flip={{ duration: 100 }}>
							<button
								class={twMerge(
									'w-full text-2xs text-left py-2 px-3 hover:bg-surface-hover transition-all whitespace-nowrap flex flex-row gap-2 items-center',
									owner === selected?.name ? 'bg-surface-hover' : ''
								)}
								on:click={() => {
									selected = selected?.name == owner ? undefined : { kind: 'owner', name: owner }
								}}
							>
								<Folder class="mr-0.5" size={14} />
								{owner}
							</button>
						</div>
					{/each}
				{:else}
					<div class="w-full text-2xs text-left py-2 px-3 gap-2 items-center"
						>No folders containing {kind} found</div
					>
				{/if}
			{/if}

			{#if preFilter === 'hub' || preFilter === 'all'}
				{#if preFilter == 'all'}
					<div class="pt-2 pb-0 text-2xs text-secondary ml-2">Integrations</div>
				{/if}
				<ListFiltersQuick {syncQuery} filters={apps} bind:selectedFilter={selected} resourceType />
			{/if}
		{:else if selectedKind === 'flow'}
			{#if owners.length > 0}
				{#each owners as owner (owner)}
					<div in:fade={{ duration: 50 }} animate:flip={{ duration: 100 }}>
						<button
							class={twMerge(
								'w-full text-2xs text-left py-2 px-3 hover:bg-surface-hover transition-all whitespace-nowrap flex flex-row gap-2 items-center',
								owner === selected?.name ? 'bg-surface-hover' : ''
							)}
							on:click={() => {
								selected = selected?.name == owner ? undefined : { kind: 'owner', name: owner }
							}}
						>
							<Folder class="mr-0.5" size={14} />
							{owner}
						</button>
					</div>
				{/each}
			{/if}
		{/if}
	</div>
	<div class="flex-grow h-full overflow-y-auto">
		{#if ['action', 'trigger', 'approval'].includes(selectedKind)}
			{#if !disableAi && funcDesc.length > 0}
				<ul class="transition-all divide-y">
					<li>
						<button
							class="py-2 gap-4 flex flex-row hover:bg-surface-hover transition-all items-center justify-between w-full"
							on:click={() => {
								lang = 'bun'
								onGenerate()
								close()
							}}
						>
							<div class="flex items-center gap-2.5 px-2">
								<div
									class="rounded-md p-1 flex justify-center items-center bg-surface border w-6 h-6"
								>
									<Wand2 size={14} class="text-violet-800 dark:text-violet-400" />
								</div>
								<div class="text-left text-xs text-secondary">
									Generate "{funcDesc}" in TypeScript
								</div>
							</div>
						</button>
					</li>
					<li>
						<button
							class="py-2 gap-4 flex flex-row hover:bg-surface-hover transition-all items-center justify-between w-full"
							on:click={() => {
								lang = 'python3'
								onGenerate()
								close()
							}}
						>
							<div class="flex items-center gap-2.5 px-2">
								<div
									class="rounded-md p-1 flex justify-center items-center bg-surface border w-6 h-6"
								>
									<Wand2 size={14} class="text-violet-800 dark:text-violet-400" />
								</div>

								<div class="text-left text-xs text-secondary">
									Generate "{funcDesc}" in Python
								</div>
							</div>
						</button>
					</li>
				</ul>
			{/if}
			{#if funcDesc.length === 0 && (!selected || selected?.kind === 'inline') && preFilter == 'all'}
				<div class="pt-2 pb-0 text-2xs text-secondary ml-2">Create Inline Script</div>
				{#each langs.filter((lang) => customUi?.languages == undefined || customUi?.languages?.includes(lang?.[1])) as [label, lang] (lang)}
					{#if displayLang(lang, kind)}
						<FlowScriptPickerQuick
							{label}
							lang={lang == 'docker' ? 'bash' : lang}
							on:click={() => {
								if (lang == 'docker') {
									if (isCloudHosted()) {
										sendUserToast(
											'You cannot use Docker scripts on the multi-tenant platform. Use a dedicated instance or self-host windmill instead.',
											true,
											[
												{
													label: 'Learn more',
													callback: () => {
														window.open('https://www.windmill.dev/docs/advanced/docker', '_blank')
													}
												}
											]
										)
										return
									}
								}

								dispatch('new', {
									kind: 'script',
									inlineScript: {
										language: lang == 'docker' ? 'bash' : lang,
										kind,
										subkind: lang == 'docker' ? 'docker' : 'flow',
										summary
									}
								})
							}}
						/>
					{/if}
				{/each}
			{/if}
			{#if (!selected || selected?.kind === 'integrations') && (preFilter === 'hub' || preFilter === 'all')}
				{#if !selected && preFilter !== 'hub'}
					<div class="pt-2 pb-0 text-2xs text-secondary ml-2">Hub</div>
				{/if}
				<PickHubScriptQuick
					bind:filter
					bind:apps
					appFilter={selected?.name}
					{kind}
					on:pickScript
					bind:loading
				/>
			{/if}

			{#if (!selected || selected?.kind === 'owner') && (preFilter === 'workspace' || preFilter === 'all')}
				{#if !selected && preFilter !== 'workspace'}
					<div class="pb-0 text-2xs text-secondary ml-2">Workspace</div>
				{/if}
				<WorkspaceScriptPickerQuick
					bind:owners
					bind:ownerFilter={selected}
					{filter}
					{kind}
					on:pickScript
				/>
			{/if}

			{#if funcDesc.length > 0 && filteredItems.length > 0}
				<div class="text-left mt-2">
					<p class="text-xs text-secondary ml-2">Workspace {trigger ? 'Triggers' : 'Scripts'}</p>
					<ul class="transition-all">
						{#each filteredItems.slice(0, 3) as item (item.path)}
							<li>
								<button
									class="py-2 gap-4 flex flex-row hover:bg-surface-hover transition-all items-center justify-between w-full"
									on:click={() => {
										dispatch('insert', { path: item.path, summary: item.summary })
										close()
									}}
								>
									<div class="flex items-center gap-2.5 px-2">
										<div
											class="rounded-md p-1 flex justify-center items-center bg-surface border w-6 h-6"
										>
											<svelte:component this={APP_TO_ICON_COMPONENT[item['app']]} />
										</div>

										<div class="text-left text-xs text-secondary">
											{defaultIfEmptyString(item.summary, item.path)}
										</div>
									</div>
								</button>
							</li>
						{/each}
					</ul>
				</div>
			{/if}
		{:else if selectedKind === 'flow'}
			<FlowInputsFlowQuick bind:owners {filter} on:pickFlow />
		{/if}
	</div>
</div>
