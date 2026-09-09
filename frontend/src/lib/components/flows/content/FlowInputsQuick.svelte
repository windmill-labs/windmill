<script module lang="ts">
	let cachedOwners: Record<string, string[]> = {}
	let cachedIntegrations: string[] = []
</script>

<script lang="ts">
	import { sendUserToast } from '$lib/toast'
	import FlowScriptPickerQuick from '../pickers/FlowScriptPickerQuick.svelte'
	import { defaultScriptLanguages, processInlineLangs } from '$lib/scripts'
	import {
		defaultScripts,
		enterpriseLicense,
		hubBaseUrlStore,
		userStore,
		workspaceStore
	} from '$lib/stores'
	import type { SupportedLanguage } from '$lib/common'
	import { createEventDispatcher, getContext, untrack } from 'svelte'
	import type { FlowBuilderWhitelabelCustomUi } from '$lib/components/custom_ui'
	import { type Script, type ScriptLang, type HubScriptKind } from '$lib/gen'
	import ListFiltersQuick from '$lib/components/home/ListFiltersQuick.svelte'
	import { ExternalLink, Folder, User, X } from 'lucide-svelte'
	import { fade } from 'svelte/transition'
	import { flip } from 'svelte/animate'
	import { Button } from '$lib/components/common'
	import { SettingsIcon } from 'lucide-svelte'
	import DefaultScriptsInner from '$lib/components/DefaultScriptsInner.svelte'
	import GenAiQuick from './GenAiQuick.svelte'
	import FlowToplevelNode from '../pickers/FlowToplevelNode.svelte'
	import { copilotInfo } from '$lib/aiStore'
	import {
		canHavePreprocessor,
		canHaveTrigger,
		canHaveApproval,
		canHaveFailure
	} from '$lib/script_helpers'

	const dispatch = createEventDispatcher()

	interface Props {
		summary?: string | undefined
		filter?: string
		disableAi?: boolean
		preFilter?: 'all' | 'workspace' | 'hub'
		funcDesc: string
		owners?: string[]
		loading?: boolean
		kind: 'trigger' | 'script' | 'preprocessor' | 'failure' | 'approval'
		selectedKind?: 'script' | 'flow' | 'approval' | 'trigger' | 'preprocessor' | 'failure'
		displayPath?: boolean
		refreshCount?: number
	}

	let {
		summary = undefined,
		filter = $bindable(''),
		disableAi = false,
		preFilter = 'hub',
		funcDesc,
		owners = $bindable([]),
		loading = $bindable(false),
		kind,
		selectedKind = kind,
		displayPath = false,
		refreshCount = 0
	}: Props = $props()

	if ($workspaceStore && cachedOwners?.[$workspaceStore]) {
		owners = cachedOwners[$workspaceStore]
	}
	type HubCompletion = {
		path: string
		summary: string
		/** The hub's own wording, before `summary` is rewritten as the display label. */
		hubSummary: string
		id: number
		version_id: number
		ask_id: number
		app: string
		kind: HubScriptKind
	}

	let lang: ScriptLang | undefined = $state(undefined)

	let filteredWorkspaceItems: (Script & { marked?: string })[] = $state([])

	let hubCompletions: HubCompletion[] = $state([])

	let selected: { kind: 'owner' | 'integrations'; name: string | undefined } | undefined =
		$state(undefined)

	let integrations: string[] = $state(cachedIntegrations)

	let customUi: undefined | FlowBuilderWhitelabelCustomUi = getContext('customUi')

	function displayLang(
		lang: SupportedLanguage | 'docker',
		kind: 'script' | 'flow' | 'approval' | 'trigger' | 'preprocessor' | 'failure'
	) {
		if (kind == 'trigger') {
			return canHaveTrigger(lang as SupportedLanguage)
		} else if (kind == 'script') {
			return true
		} else if (kind == 'approval') {
			return canHaveApproval(lang as SupportedLanguage)
		} else if (kind == 'flow') {
			return false
		} else if (kind == 'preprocessor') {
			return canHavePreprocessor(lang as SupportedLanguage)
		} else if (kind == 'failure') {
			return canHaveFailure(lang as SupportedLanguage)
		}
	}

	async function onGenerate() {
		if (!$copilotInfo.enabled) {
			sendUserToast(
				'Windmill AI is not enabled, you can activate it in the workspace settings',
				true
			)
			return
		}
		console.log('ongenerate', selectedKind, lang, funcDesc)
		dispatch('new', {
			kind: selectedKind,
			inlineScript: {
				language: lang,
				kind: selectedKind,
				subkind: 'flow',
				summary,
				instructions: funcDesc
			}
		})
	}

	let openScriptSettings = $state(false)

	let selectedByKeyboard = $state(0)

	function onSelectedKindChange(
		_selectedKind: 'script' | 'flow' | 'approval' | 'trigger' | 'preprocessor' | 'failure'
	) {
		selectedByKeyboard = 0
	}

	let inlineScripts: [string, SupportedLanguage | 'docker'][] = $state([])

	const enterpriseLangs = ['mssql', 'oracledb']

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
		['Branch to all', 'branchall'],
		...(customUi?.aiAgent != false ? ([['AI Agent', 'aiagent']] as [string, string][]) : [])
	]

	let topLevelNodes: [string, string][] = $state([])
	function computeToplevelNodeChoices(funcDesc: string, preFilter: 'all' | 'workspace' | 'hub') {
		if (funcDesc.length > 0 && preFilter == 'all' && kind == 'script') {
			topLevelNodes = allToplevelNodes.filter((node) =>
				node[0].toLowerCase().startsWith(funcDesc.toLowerCase())
			)
		} else {
			topLevelNodes = []
		}
	}

	function onPrefilterChange(preFilter: 'all' | 'workspace' | 'hub') {
		if (preFilter == 'workspace') {
			hubCompletions = []
		} else if (preFilter == 'hub') {
			filteredWorkspaceItems = []
		}
		selectedByKeyboard = 0
	}

	let scrollable: HTMLElement | undefined = $state()
	function onKeyDown(e: KeyboardEvent) {
		let length = hubOffset + (hubCompletions?.length ?? 0)
		if (e.key === 'ArrowDown') {
			selectedByKeyboard = (selectedByKeyboard + 1) % length
			scrollable?.scrollTo({ top: selectedByKeyboard * 32, behavior: 'smooth' })
			e.preventDefault()
		} else if (e.key === 'ArrowUp') {
			selectedByKeyboard = (selectedByKeyboard - 1 + length) % length
			scrollable?.scrollTo({ top: selectedByKeyboard * 32, behavior: 'smooth' })
			e.preventDefault()
		}
	}

	// Mouse and keyboard share this one index, so rows report hover on mousemove rather than
	// mouseenter: arrow keys scroll the list, which slides a row under a stationary cursor and
	// fires mouseenter, which would otherwise hijack the selection mid-navigation.
	function hover(index: number) {
		selectedByKeyboard = index
	}

	let langs = $derived(
		processInlineLangs(undefined, $defaultScripts?.order ?? Object.keys(defaultScriptLanguages))
			.map((l) => [defaultScriptLanguages[l], l])
			.filter(
				(x) => $defaultScripts?.hidden == undefined || !$defaultScripts.hidden.includes(x[1])
			) as [string, SupportedLanguage | 'docker'][]
	)
	$effect(() => {
		selectedKind
		untrack(() => onSelectedKindChange(selectedKind))
	})
	$effect(() => {
		;[funcDesc, preFilter]
		untrack(() => computeToplevelNodeChoices(funcDesc, preFilter))
	})
	$effect(() => {
		;[funcDesc, selected, preFilter, selectedKind]
		untrack(() => computeInlineScriptChoices(funcDesc, selected, preFilter, selectedKind))
	})
	$effect(() => {
		preFilter
		untrack(() => onPrefilterChange(preFilter))
	})
	// Gates the two AI rows and their slots in the index space; both must agree or arrow keys land
	// on indices that render nothing.
	let showAiRows = $derived(
		!disableAi &&
			!$copilotInfo.workspaceDisabled &&
			funcDesc?.length > 0 &&
			kind != 'failure' &&
			kind != 'preprocessor' &&
			(selectedKind == 'script' || selectedKind == 'trigger') &&
			preFilter == 'all'
	)
	let aiLength = $derived(showAiRows ? 2 : 0)

	// Must match the heading and label rendered for the AI Sandbox section
	const aiSandboxHeading = 'AI Sandbox'
	const aiSandboxLabel = 'Claude Code'
	let matchesAiSandbox = $derived.by(() => {
		const query = funcDesc?.trim().toLowerCase() ?? ''
		if (query.length == 0) {
			return true
		}
		return [aiSandboxHeading, aiSandboxLabel].some((term) => {
			const lowered = term.toLowerCase()
			return lowered.startsWith(query) || lowered.split(' ').some((w) => w.startsWith(query))
		})
	})
	// Gates the AI Sandbox row and its slot in the index space; both must agree or arrow keys land
	// on an index that renders nothing.
	let showAiSandbox = $derived(
		selectedKind === 'script' &&
			preFilter === 'all' &&
			!selected &&
			customUi?.aiSandbox != false &&
			matchesAiSandbox
	)

	// Every result row lives in one keyboard index space, and hovering a row moves that index, so
	// mouse and keyboard can never highlight two different rows. Offsets follow the render order.
	let inlineOffset = $derived(topLevelNodes.length)
	let aiOffset = $derived(inlineOffset + (inlineScripts?.length ?? 0))
	let workspaceOffset = $derived(aiOffset + aiLength)
	let aiSandboxOffset = $derived(workspaceOffset + (filteredWorkspaceItems?.length ?? 0))
	let hubOffset = $derived(aiSandboxOffset + (showAiSandbox ? 1 : 0))
</script>

<svelte:window onkeydown={onKeyDown} />
<div class="flex flex-row grow min-w-0 divide-x relative bg-surface-tertiary rounded-md">
	{#if selectedKind != 'preprocessor'}
		<div class="h-full overflow-auto p-2 w-36 shrink-0 gap-1 flex flex-col">
			{#if ['script', 'trigger', 'approval', 'preprocessor', 'failure'].includes(selectedKind)}
				{#if (preFilter === 'all' && owners.length > 0) || preFilter === 'workspace'}
					{#if preFilter !== 'workspace'}
						<div class="pb-0 text-2xs font-normal text-secondary ml-2">Folders</div>
					{/if}

					{#if owners.length > 0}
						{#each owners as owner (owner)}
							<div
								in:fade={{ duration: 50 }}
								animate:flip={{ duration: 100 }}
								class="w-full px-0.5"
							>
								<Button
									selected={owner === selected?.name}
									onClick={() => {
										selected = selected?.name == owner ? undefined : { kind: 'owner', name: owner }
									}}
									variant="subtle"
									unifiedSize="sm"
									btnClasses="justify-start"
									startIcon={{
										icon: owner.startsWith('f/') ? Folder : User,
										props: { width: 14, height: 14 }
									}}
									title={owner.slice(2)}
								>
									<span class="truncate">{owner.slice(2)}</span>
								</Button>
							</div>
						{/each}
						<div class="pb-1"></div>
					{:else}
						<div class="text-2xs text-primary font-normal text-center py-3 px-3 items-center">
							No items found.
						</div>
					{/if}
				{/if}

				{#if preFilter === 'hub' || preFilter === 'all'}
					{#if preFilter == 'all'}
						<div class="pb-0 text-2xs font-normal text-secondary ml-2 pt-1">Integrations</div>
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
					{#if !selected && customUi?.suggestIntegration != false}
						<div class="pl-2 py-1">
							<a
								href={`${$hubBaseUrlStore}?suggest_integration=true`}
								target="_blank"
								class="text-2xs flex flex-row items-center gap-1"
								>Suggest integration <ExternalLink class="size-3" />
							</a>
						</div>
					{/if}
				{/if}
			{:else if selectedKind === 'flow'}
				{#if owners.length > 0}
					{#each owners as owner (owner)}
						<div in:fade={{ duration: 50 }} animate:flip={{ duration: 100 }}>
							<Button
								selected={owner === selected?.name}
								variant="subtle"
								unifiedSize="sm"
								btnClasses="justify-start"
								startIcon={{
									icon: owner.startsWith('f/') ? Folder : User
								}}
								onClick={() => {
									selected = selected?.name == owner ? undefined : { kind: 'owner', name: owner }
								}}
							>
								{owner.slice(2)}
							</Button>
						</div>
					{/each}
				{/if}
			{/if}
		</div>
	{/if}
	<div
		bind:this={scrollable}
		id="flow-editor-flow-atoms"
		class="h-full overflow-auto grow min-w-0 p-2 gap-1 flex flex-col"
	>
		{#if kind == 'script'}
			{#each topLevelNodes as [label, kind], i (label)}
				<FlowToplevelNode
					on:click={() => {
						dispatch('new', { kind })
					}}
					{label}
					selected={selectedByKeyboard === i}
					onHover={() => hover(i)}
				/>
			{/each}
		{/if}

		{#if inlineScripts?.length > 0}
			<div class="pb-0 flex flex-row items-center gap-2">
				<div class="text-2xs font-normal text-secondary ml-2"
					>New {selectedKind != 'script' ? selectedKind + ' ' : ''}script</div
				>
				{#if $userStore?.is_admin || $userStore?.is_super_admin}
					{#if !openScriptSettings}
						<Button
							onClick={() => (openScriptSettings = true)}
							startIcon={{ icon: SettingsIcon }}
							unifiedSize="sm"
							variant="subtle"
							title="Edit global default scripts"
							btnClasses="-my-3"
						/>
					{:else}
						<Button
							onClick={() => (openScriptSettings = false)}
							startIcon={{ icon: X }}
							variant="accent"
							unifiedSize="sm"
							btnClasses="-my-3"
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
					selected={selectedByKeyboard === i + inlineOffset}
					onHover={() => hover(i + inlineOffset)}
					{enterpriseLangs}
					{label}
					lang={lang == 'docker' ? 'bash' : lang}
					on:click={() => {
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

		{#if showAiRows}
			<ul class="transition-all">
				<li
					><GenAiQuick
						{funcDesc}
						lang="TypeScript"
						selected={selectedByKeyboard === aiOffset}
						onHover={() => hover(aiOffset)}
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
						selected={selectedByKeyboard === aiOffset + 1}
						onHover={() => hover(aiOffset + 1)}
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
				<div class="pt-2 pb-0 text-2xs font-normal text-secondary ml-2">Workspace</div>
			{/if}
			{#await import('../pickers/WorkspaceScriptPickerQuick.svelte') then Module}
				<Module.default
					bind:owners={
						() => owners,
						(v) => {
							$workspaceStore && (cachedOwners[$workspaceStore] = v)
							owners = v
						}
					}
					bind:ownerFilter={selected}
					bind:filteredWithOwner={filteredWorkspaceItems}
					{filter}
					kind={selectedKind}
					selected={selectedByKeyboard - workspaceOffset}
					on:pickScript
					on:pickFlow
					{displayPath}
					{refreshCount}
					onHover={(i) => hover(workspaceOffset + i)}
				/>
			{/await}
			<div class="pb-1"></div>
		{/if}
		{#if showAiSandbox}
			<div class="pb-0 text-2xs font-normal text-secondary ml-2">{aiSandboxHeading}</div>
			<FlowScriptPickerQuick
				eeRestricted={false}
				selected={selectedByKeyboard === aiSandboxOffset}
				onHover={() => hover(aiSandboxOffset)}
				enterpriseLangs={[]}
				label={aiSandboxLabel}
				lang="claudesandbox"
				on:click={() => {
					dispatch('new', {
						kind: selectedKind,
						inlineScript: {
							language: 'bun',
							kind: selectedKind,
							subkind: 'claudesandbox',
							summary
						}
					})
				}}
			/>
		{/if}
		{#if selectedKind != 'preprocessor' && selectedKind != 'flow'}
			{#if (!selected || selected?.kind === 'integrations') && (preFilter === 'hub' || preFilter === 'all')}
				{#if !selected && preFilter !== 'hub'}
					<div class=" pb-0 text-2xs font-normal text-secondary ml-2">Hub</div>
				{/if}
				{#await import('../pickers/PickHubScriptQuick.svelte') then Module}
					<Module.default
						bind:items={hubCompletions}
						bind:filter
						bind:apps={
							() => integrations,
							(v) => {
								cachedIntegrations = v
								integrations = v
							}
						}
						appFilter={selected?.name}
						kind={selectedKind}
						selected={selectedByKeyboard - hubOffset}
						on:pickScript
						bind:loading
						{displayPath}
						{refreshCount}
						onHover={(i) => hover(hubOffset + i)}
					/>
				{/await}
			{/if}
		{/if}
	</div>
</div>
