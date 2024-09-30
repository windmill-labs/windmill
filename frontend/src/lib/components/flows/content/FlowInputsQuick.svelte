<script lang="ts">
	import { isCloudHosted } from '$lib/cloud'
	import { sendUserToast } from '$lib/toast'
	import FlowScriptPickerQuick from '../pickers/FlowScriptPickerQuick.svelte'
	import WorkspaceScriptPickerQuick from '../pickers/WorkspaceScriptPickerQuick.svelte'
	import { defaultScriptLanguages, processLangs } from '$lib/scripts'
	import { defaultScripts, enterpriseLicense, userStore } from '$lib/stores'
	import type { SupportedLanguage } from '$lib/common'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { FlowBuilderWhitelabelCustomUi } from '$lib/components/custom_ui'
	import PickHubScriptQuick from '../pickers/PickHubScriptQuick.svelte'
	import { type Script, type FlowModule } from '$lib/gen'
	import ListFiltersQuick from '$lib/components/home/ListFiltersQuick.svelte'
	import { Folder, User } from 'lucide-svelte'
	import type { FlowCopilotContext, FlowCopilotModule } from '../../copilot/flow'
	import type { FlowEditorContext } from '../../flows/types'
	import { copilotInfo } from '$lib/stores'
	import { nextId } from '../../flows/flowModuleNextId'
	import { twMerge } from 'tailwind-merge'
	import { fade } from 'svelte/transition'
	import { flip } from 'svelte/animate'
	import FlowInputsFlowQuick from '../content/FlowInputsFlowQuick.svelte'
	import Scrollable from '$lib/components/Scrollable.svelte'
	import { Button } from '$lib/components/common'
	import { SettingsIcon } from 'lucide-svelte'
	import DefaultScriptsInner from '$lib/components/DefaultScriptsInner.svelte'
	import GenAiQuick from './GenAiQuick.svelte'

	const dispatch = createEventDispatcher()

	export let failureModule: boolean
	export let preprocessorModule: boolean
	export let summary: string | undefined = undefined
	export let filter = ''
	export let disableAi = false
	export let syncQuery = false
	export let preFilter: 'all' | 'workspace' | 'hub' = 'hub'
	export let funcDesc: string
	export let index: number
	export let modules: FlowModule[]
	export let owners: string[] = []
	export let loading = false
	export let small = false

	let trigger = false
	let lang: FlowCopilotModule['lang'] = undefined
	let selectedCompletion: FlowCopilotModule['selectedCompletion'] = undefined

	let filteredItems: (Script & { marked?: string })[] = []

	let hubCompletions: FlowCopilotModule['hubCompletions'] = []

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

	let ownerShort: string = ''

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
			return (kind == 'script' || kind == 'trigger' || failureModule) && !preprocessorModule
		}

		if (lang == 'bash' || lang == 'nativets') {
			return kind == 'script' && !preprocessorModule
		}
		return kind == 'script' && !failureModule && !preprocessorModule
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

	let openScriptSettings = false

	let selectedByKeyboard = 0

	let inlineScripts: [string, SupportedLanguage | 'docker'][] = []

	const enterpriseLangs = ['bigquery', 'snowflake', 'mssql']

	function computeInlineScriptChoices(funcDesc: string) {
		if (['action', 'trigger', 'approval'].includes(selectedKind)) {
			if ((!selected || selected?.kind === 'inline') && preFilter == 'all') {
				inlineScripts = langs.filter(
					(lang) =>
						((customUi?.languages == undefined || customUi?.languages?.includes(lang?.[1])) &&
							(funcDesc?.length == 0 ||
								lang?.[0]?.toLowerCase()?.includes(funcDesc?.toLowerCase())) &&
							displayLang(lang?.[1], kind) &&
							!enterpriseLangs.includes(lang?.[1] || '')) ||
						!!$enterpriseLicense
				)
			}
		}
	}

	$: computeInlineScriptChoices(funcDesc)

	$: aiLength =
		funcDesc?.length > 0 && !disableAi && selectedKind != 'flow' && preFilter == 'all' ? 2 : 0

	let scrollable: Scrollable | undefined
	function onKeyDown(e: KeyboardEvent) {
		let length = inlineScripts.length + aiLength + filteredItems.length + hubCompletions.length
		if (e.key === 'ArrowDown') {
			selectedByKeyboard = (selectedByKeyboard + 1) % length

			scrollable?.scrollIntoView(selectedByKeyboard * 32)
		} else if (e.key === 'ArrowUp') {
			selectedByKeyboard = (selectedByKeyboard - 1 + length) % length
			scrollable?.scrollIntoView(selectedByKeyboard * 32)
		}
		e.preventDefault()
	}
</script>

<svelte:window on:keydown={onKeyDown} />
<div
	class="flex flex-row grow min-w-0 divide-x {!small
		? 'shadow-[inset_25px_0px_12px_-30px_rgba(94,129,172,0.5)]'
		: ''}"
>
	<Scrollable scrollableClass="w-32 grow-0 shrink-0 ">
		{#if ['action', 'trigger', 'approval'].includes(selectedKind)}
			{#if (preFilter === 'all' && owners.length > 0) || preFilter === 'workspace'}
				{#if preFilter !== 'workspace'}
					<div class="pb-0 text-2xs font-light text-secondary ml-2">Workspace Folders</div>
				{/if}
				{#if owners.length > 0}
					{#each owners as owner (owner)}
						<div in:fade={{ duration: 50 }} animate:flip={{ duration: 100 }}>
							<button
								class={twMerge(
									'w-full  text-left text-2xs text-primary font-normal py-2 px-3 hover:bg-surface-hover transition-all whitespace-nowrap flex flex-row gap-2 items-center rounded-md',
									owner === selected?.name ? 'bg-surface-hover' : ''
								)}
								on:click={() => {
									selected = selected?.name == owner ? undefined : { kind: 'owner', name: owner }
								}}
							>
								{#if owner.startsWith('f/')}
									<Folder class="mr-0.5" size={14} />
								{:else}
									<User class="mr-0.5" size={14} />
								{/if}
								{(ownerShort = owner).slice(2)}
							</button>
						</div>
					{/each}
				{:else}
					<div class="text-2xs text-tertiary font-light text-center py-3 px-3 items-center">
						No items found.
					</div>
				{/if}
			{/if}

			{#if preFilter === 'hub' || preFilter === 'all'}
				{#if preFilter == 'all'}
					<div class="pt-2 pb-0 text-2xs font-light text-secondary ml-2">Integrations</div>
				{/if}
				<ListFiltersQuick {syncQuery} filters={apps} bind:selectedFilter={selected} resourceType />
			{/if}
		{:else if selectedKind === 'flow'}
			{#if owners.length > 0}
				{#each owners as owner (owner)}
					<div in:fade={{ duration: 50 }} animate:flip={{ duration: 100 }}>
						<button
							class={twMerge(
								'w-full text-left text-2xs text-primary font-normal py-2 px-3 hover:bg-surface-hover transition-all whitespace-nowrap flex flex-row gap-2 items-center rounded-md',
								owner === selected?.name ? 'bg-surface-hover' : ''
							)}
							on:click={() => {
								selected = selected?.name == owner ? undefined : { kind: 'owner', name: owner }
							}}
						>
							{#if owner.startsWith('f/')}
								<Folder class="mr-0.5" size={14} />
							{:else}
								<User class="mr-0.5" size={14} />
							{/if}
							{(ownerShort = owner).slice(2)}
						</button>
					</div>
				{/each}
			{/if}
		{/if}
	</Scrollable>

	<Scrollable bind:this={scrollable} scrollableClass="grow min-w-0">
		{#if inlineScripts?.length > 0}
			<div class="pb-0 flex flex-row items-center gap-2">
				<div class=" text-2xs font-light text-secondary ml-2">New Inline Script</div>
				{#if $userStore?.is_admin || $userStore?.is_super_admin}
					{#if !openScriptSettings}
						<Button
							on:click={() => (openScriptSettings = true)}
							startIcon={{ icon: SettingsIcon }}
							color="light"
							size="xs2"
							btnClasses="!text-tertiary"
							variant="contained"
						/>
					{:else}
						<Button
							on:click={() => (openScriptSettings = false)}
							startIcon={{ icon: SettingsIcon }}
							color="dark"
							size="xs2"
							variant="contained"
						>
							Close
						</Button>
					{/if}
				{/if}
			</div>
			{#if openScriptSettings}
				<div class="p-2">
					<DefaultScriptsInner small />
				</div>
			{/if}
			{#each inlineScripts as [label, lang], i (lang)}
				<FlowScriptPickerQuick
					selected={selectedByKeyboard === i}
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
								subkind: lang == 'docker' ? 'docker' : preprocessorModule ? 'preprocessor' : 'flow',
								summary
							}
						})
					}}
				/>
			{/each}
		{/if}

		{#if !disableAi && funcDesc?.length > 0}
			<ul class="transition-all">
				<li
					><GenAiQuick
						{funcDesc}
						lang="TypeScript"
						selected={selectedByKeyboard === inlineScripts?.length + 0}
						on:click={() => {
							lang = 'bun'
							onGenerate()
							close()
						}}
					/>
				</li>
				<li>
					<GenAiQuick
						{funcDesc}
						lang="Python"
						selected={selectedByKeyboard === inlineScripts?.length + 1}
						on:click={() => {
							lang = 'python3'
							onGenerate()
							close()
						}}
					/>
				</li>
			</ul>
		{/if}

		{#if (!selected || selected?.kind === 'owner') && (preFilter === 'workspace' || preFilter === 'all')}
			{#if !selected && (preFilter !== 'workspace' || funcDesc?.length > 0)}
				<div class="pt-2 pb-0 text-2xs font-light text-secondary ml-2">Workspace</div>
			{/if}
			<WorkspaceScriptPickerQuick
				bind:owners
				bind:ownerFilter={selected}
				bind:filteredItems
				{filter}
				{kind}
				selected={selectedByKeyboard - inlineScripts?.length - aiLength}
				on:pickScript
			/>
		{/if}

		{#if (!selected || selected?.kind === 'integrations') && (preFilter === 'hub' || preFilter === 'all')}
			{#if !selected && preFilter !== 'hub'}
				<div class=" pb-0 text-2xs font-light text-secondary ml-2">Hub</div>
			{/if}
			<PickHubScriptQuick
				bind:items={hubCompletions}
				bind:filter
				bind:apps
				appFilter={selected?.name}
				{kind}
				selected={selectedByKeyboard - inlineScripts?.length - aiLength - filteredItems?.length}
				on:pickScript
				bind:loading
			/>
		{/if}

		{#if selectedKind === 'flow'}
			<FlowInputsFlowQuick bind:owners {filter} on:pickFlow />
		{/if}
	</Scrollable>
</div>
