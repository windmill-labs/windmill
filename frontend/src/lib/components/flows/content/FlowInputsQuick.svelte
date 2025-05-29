<script lang="ts">
	import { isCloudHosted } from '$lib/cloud'
	import { sendUserToast } from '$lib/toast'
	import FlowScriptPickerQuick from '../pickers/FlowScriptPickerQuick.svelte'
	import WorkspaceScriptPickerQuick from '../pickers/WorkspaceScriptPickerQuick.svelte'
	import { defaultScriptLanguages, processLangs } from '$lib/scripts'
	import { defaultScripts, enterpriseLicense, userStore } from '$lib/stores'
	import type { SupportedLanguage } from '$lib/common'
	import { createEventDispatcher, getContext, onDestroy, onMount } from 'svelte'
	import type { FlowBuilderWhitelabelCustomUi } from '$lib/components/custom_ui'
	import PickHubScriptQuick from '../pickers/PickHubScriptQuick.svelte'
	import { type Script, type FlowModule, type ScriptLang, type HubScriptKind } from '$lib/gen'
	import ListFiltersQuick from '$lib/components/home/ListFiltersQuick.svelte'
	import { Folder, User } from 'lucide-svelte'
	import type { FlowEditorContext } from '../../flows/types'
	import { copilotInfo } from '$lib/stores'
	import { twMerge } from 'tailwind-merge'
	import { fade } from 'svelte/transition'
	import { flip } from 'svelte/animate'
	import Scrollable from '$lib/components/Scrollable.svelte'
	import { Button } from '$lib/components/common'
	import { SettingsIcon } from 'lucide-svelte'
	import DefaultScriptsInner from '$lib/components/DefaultScriptsInner.svelte'
	import GenAiQuick from './GenAiQuick.svelte'
	import FlowToplevelNode from '../pickers/FlowToplevelNode.svelte'

	const dispatch = createEventDispatcher()

	export let summary: string | undefined = undefined
	export let filter = ''
	export let disableAi = false
	export let preFilter: 'all' | 'workspace' | 'hub' = 'hub'
	export let funcDesc: string
	export let index: number
	export let modules: FlowModule[]
	export let owners: string[] = []
	export let loading = false
	export let small = false
	export let kind: 'trigger' | 'script' | 'preprocessor' | 'failure' | 'approval'
	export let selectedKind: 'script' | 'flow' | 'approval' | 'trigger' | 'preprocessor' | 'failure' =
		kind
	export let displayPath = false

	type HubCompletion = {
		path: string
		summary: string
		id: number
		version_id: number
		ask_id: number
		app: string
		kind: HubScriptKind
	}

	let lang: ScriptLang | undefined = undefined
	console.log(lang)
	let selectedCompletion: HubCompletion | undefined = undefined

	let filteredWorkspaceItems: (Script & { marked?: string })[] = []

	let hubCompletions: HubCompletion[] = []

	const { insertButtonOpen } = getContext<FlowEditorContext>('FlowEditorContext')

	let selected: { kind: 'owner' | 'integrations'; name: string | undefined } | undefined = undefined

	let integrations: string[] = []

	let customUi: undefined | FlowBuilderWhitelabelCustomUi = getContext('customUi')

	$: langs = processLangs(undefined, $defaultScripts?.order ?? Object.keys(defaultScriptLanguages))
		.map((l) => [defaultScriptLanguages[l], l])
		.filter(
			(x) => $defaultScripts?.hidden == undefined || !$defaultScripts.hidden.includes(x[1])
		) as [string, SupportedLanguage | 'docker'][]

	function displayLang(
		lang: SupportedLanguage | 'docker',
		kind: 'script' | 'flow' | 'approval' | 'trigger' | 'preprocessor' | 'failure'
	) {
		if (kind == 'trigger') {
			return ['python3', 'bun', 'deno', 'go'].includes(lang)
		} else if (kind == 'script') {
			return true
		} else if (kind == 'approval') {
			return ['python3', 'bun', 'deno'].includes(lang)
		} else if (kind == 'flow') {
			return false
		} else if (kind == 'preprocessor') {
			return ['python3', 'bun', 'deno'].includes(lang)
		} else if (kind == 'failure') {
			return ['python3', 'bun', 'deno', 'go'].includes(lang)
		}
	}

	async function onGenerate() {
		if (!selectedCompletion && !$copilotInfo.enabled) {
			sendUserToast(
				'Windmill AI is not enabled, you can activate it in the workspace settings',
				true
			)
			return
		}
		//TODO gen
		dispatch('close')
	}

	let openScriptSettings = false

	let selectedByKeyboard = 0

	$: onSelectedKindChange(selectedKind)

	function onSelectedKindChange(
		_selectedKind: 'script' | 'flow' | 'approval' | 'trigger' | 'preprocessor' | 'failure'
	) {
		selectedByKeyboard = 0
	}

	let inlineScripts: [string, SupportedLanguage | 'docker'][] = []

	const enterpriseLangs = ['bigquery', 'snowflake', 'mssql', 'oracledb']

	function computeInlineScriptChoices(
		funcDesc: string,
		selected: { kind: 'owner' | 'integrations'; name: string | undefined } | undefined,
		preFilter: 'all' | 'workspace' | 'hub',
		selectedKind: 'script' | 'flow' | 'approval' | 'trigger' | 'preprocessor' | 'failure'
	) {
		if (['script', 'trigger', 'failure', 'approval', 'preprocessor'].includes(selectedKind)) {
			if (!selected && preFilter == 'all') {
				inlineScripts = langs.filter((lang) => {
					return (
						(customUi?.languages == undefined || customUi?.languages?.includes(lang?.[1])) &&
						(funcDesc?.length == 0 ||
							lang?.[0]?.toLowerCase()?.includes(funcDesc?.toLowerCase())) &&
						displayLang(lang?.[1], selectedKind)
					)
				})
				return
			}
		}
		inlineScripts = []
	}

	const allToplevelNodes: [string, string][] = [
		['For loop', 'forloop'],
		['While loop', 'whileloop'],
		['Branch to one', 'branchone'],
		['Branch to all', 'branchall']
	]
	let topLevelNodes: [string, string][] = []
	function computeToplevelNodeChoices(funcDesc: string, preFilter: 'all' | 'workspace' | 'hub') {
		if (funcDesc.length > 0 && preFilter == 'all' && kind == 'script') {
			topLevelNodes = allToplevelNodes.filter((node) =>
				node[0].toLowerCase().startsWith(funcDesc.toLowerCase())
			)
		} else {
			topLevelNodes = []
		}
	}

	$: computeToplevelNodeChoices(funcDesc, preFilter)
	$: computeInlineScriptChoices(funcDesc, selected, preFilter, selectedKind)
	$: onPrefilterChange(preFilter)

	function onPrefilterChange(preFilter: 'all' | 'workspace' | 'hub') {
		if (preFilter == 'workspace') {
			hubCompletions = []
		} else if (preFilter == 'hub') {
			filteredWorkspaceItems = []
		}
		selectedByKeyboard = 0
	}

	$: aiLength =
		funcDesc?.length > 0 && !disableAi && selectedKind != 'flow' && preFilter == 'all' ? 2 : 0

	let scrollable: Scrollable | undefined
	function onKeyDown(e: KeyboardEvent) {
		let length =
			topLevelNodes?.length +
			inlineScripts.length +
			aiLength +
			filteredWorkspaceItems.length +
			hubCompletions.length
		if (e.key === 'ArrowDown') {
			selectedByKeyboard = (selectedByKeyboard + 1) % length
			scrollable?.scrollIntoView(selectedByKeyboard * 32)
			e.preventDefault()
		} else if (e.key === 'ArrowUp') {
			selectedByKeyboard = (selectedByKeyboard - 1 + length) % length
			scrollable?.scrollIntoView(selectedByKeyboard * 32)
			e.preventDefault()
		}
	}

	onMount(() => {
		$insertButtonOpen = true
	})

	onDestroy(() => {
		$insertButtonOpen = false
	})
</script>

<svelte:window on:keydown={onKeyDown} />
<div class="flex flex-row grow min-w-0 divide-x relative {!small ? 'shadow-inset' : ''}">
	{#if selectedKind != 'preprocessor'}
		<Scrollable shiftedShadow scrollableClass="w-32 grow-0 shrink-0 ">
			{#if ['script', 'trigger', 'approval', 'preprocessor', 'failure'].includes(selectedKind)}
				{#if (preFilter === 'all' && owners.length > 0) || preFilter === 'workspace'}
					{#if preFilter !== 'workspace'}
						<div class="pb-0 text-2xs font-light text-secondary ml-2">Folders</div>
					{/if}

					{#if owners.length > 0}
						{#each owners as owner (owner)}
							<div
								in:fade={{ duration: 50 }}
								animate:flip={{ duration: 100 }}
								class="w-full px-0.5"
							>
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
									{owner.slice(2)}
								</button>
							</div>
						{/each}
						<div class="pb-1.5"></div>
					{:else}
						<div class="text-2xs text-tertiary font-light text-center py-3 px-3 items-center">
							No items found.
						</div>
					{/if}
				{/if}

				{#if preFilter === 'hub' || preFilter === 'all'}
					{#if preFilter == 'all'}
						<div class="pb-0 text-2xs font-light text-secondary ml-2 pt-0.5">Integrations</div>
					{/if}
					<ListFiltersQuick
						on:selected={() => {
							filteredWorkspaceItems = []
							selectedByKeyboard = 0
						}}
						filters={integrations}
						bind:selectedFilter={selected}
						resourceType
					/>
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
								{owner.slice(2)}
							</button>
						</div>
					{/each}
				{/if}
			{/if}
		</Scrollable>
	{/if}
	<Scrollable id="flow-editor-flow-atoms" bind:this={scrollable} scrollableClass="grow min-w-0">
		{#if kind == 'script'}
			{#each topLevelNodes as [label, kind], i (label)}
				<FlowToplevelNode
					on:click={() => {
						dispatch('new', { kind })
					}}
					{label}
					selected={selectedByKeyboard === i}
				/>
			{/each}
		{/if}

		{#if inlineScripts?.length > 0}
			<div class="pb-0 flex flex-row items-center gap-2 -mt-[3px]">
				<div class=" text-2xs font-light text-secondary ml-2"
					>New {selectedKind != 'script' ? selectedKind + ' ' : ''}script</div
				>
				{#if $userStore?.is_admin || $userStore?.is_super_admin}
					{#if !openScriptSettings}
						<Button
							on:click={() => (openScriptSettings = true)}
							startIcon={{ icon: SettingsIcon }}
							color="light"
							size="xs2"
							btnClasses="!text-tertiary"
							variant="contained"
							title="Edit global default scripts"
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
					eeRestricted={!$enterpriseLicense && enterpriseLangs.includes(lang)}
					selected={selectedByKeyboard === i + topLevelNodes.length}
					{enterpriseLangs}
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
							kind: selectedKind,
							inlineScript: {
								language: lang == 'docker' ? 'bash' : lang,
								kind: selectedKind,
								subkind:
									lang == 'docker'
										? 'docker'
										: selectedKind == 'preprocessor'
											? 'preprocessor'
											: 'flow',
								summary
							}
						})
					}}
				/>
			{/each}
		{/if}

		{#if !disableAi && funcDesc?.length > 0 && kind != 'failure' && kind != 'preprocessor' && (selectedKind == 'script' || selectedKind == 'trigger') && preFilter == 'all'}
			<ul class="transition-all">
				<li
					><GenAiQuick
						{funcDesc}
						lang="TypeScript"
						selected={selectedByKeyboard === inlineScripts?.length + topLevelNodes.length}
						on:click={() => {
							lang = 'bun'
							onGenerate()
						}}
					/>
				</li>
				<li>
					<GenAiQuick
						{funcDesc}
						lang="Python"
						selected={selectedByKeyboard === inlineScripts?.length + topLevelNodes.length + 1}
						on:click={() => {
							lang = 'python3'
							onGenerate()
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
				bind:filteredWithOwner={filteredWorkspaceItems}
				{filter}
				kind={selectedKind}
				selected={selectedByKeyboard - inlineScripts?.length - aiLength - topLevelNodes.length}
				on:pickScript
				on:pickFlow
				{displayPath}
			/>
		{/if}

		{#if selectedKind != 'preprocessor' && selectedKind != 'flow'}
			{#if (!selected || selected?.kind === 'integrations') && (preFilter === 'hub' || preFilter === 'all')}
				{#if !selected && preFilter !== 'hub'}
					<div class=" pb-0 text-2xs font-light text-secondary ml-2">Hub</div>
				{/if}

				<PickHubScriptQuick
					bind:items={hubCompletions}
					bind:filter
					bind:apps={integrations}
					appFilter={selected?.name}
					kind={selectedKind}
					selected={selectedByKeyboard -
						inlineScripts?.length -
						aiLength -
						filteredWorkspaceItems?.length -
						topLevelNodes.length}
					on:pickScript
					bind:loading
					{displayPath}
				/>
			{/if}
		{/if}
	</Scrollable>
</div>

<style>
	.shadow-inset::before {
		box-shadow: inset 25px 0px 12px -30px rgba(94, 129, 172, 0.5);
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		content: '';
		pointer-events: none;
	}
</style>
