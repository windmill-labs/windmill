<script lang="ts" module>
	import type { ComponentType } from 'svelte'
	import type { SupportedLanguage } from '$lib/common'
	import type { AssetKind, NewSchedule } from '$lib/gen'

	export type PipelineInsertKind = {
		// Machine-readable id; drives which right-column panel renders.
		id: string
		label: string
		description?: string
		icon?: ComponentType
		// Its trigger is configured in its own editor right after the script is
		// created, so the wizard's last button reads "Configure".
		configuredAfterCreate?: boolean
	}

	export type PipelineInsertPick = {
		kindId: string
		language?: SupportedLanguage
		path?: string
		// Picked output asset kind. Optional because some menu instances
		// (those without `pickOutputKind`) skip that stage entirely.
		outputKind?: string
		// Optional natural-language prompt entered on the path stage.
		// When set, the caller is expected to bootstrap the script body
		// from this prompt via AI (using language + outputKind + the
		// upstream input as context) instead of using the seeded template.
		aiPrompt?: string
		// Set by the schedule step when the picked kind is `schedule`.
		schedule?: PipelineInsertSchedule
		// Set by the asset step for the table output kinds, replacing the
		// generated name.
		outputAsset?: { kind: AssetKind; path: string }
	}

	export type PipelineInsertOptions = Pick<PipelineInsertPick, 'schedule' | 'outputAsset'>

	// Everything but the target, which is the script being created. An empty
	// `path` means the default `<script>_schedule`.
	export type PipelineInsertSchedule = Omit<NewSchedule, 'script_path'>
</script>

<script lang="ts">
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { copilotInfo } from '$lib/aiStore'
	import LanguageIcon from '$lib/components/common/languageIcons/LanguageIcon.svelte'
	import { twMerge } from 'tailwind-merge'
	import {
		compatibleOutputKinds,
		PIPELINE_OUTPUT_KINDS,
		type PipelineOutputKind
	} from './pipelineTemplates'
	import CronInput from '$lib/components/CronInput.svelte'
	import type { ScriptLang } from '$lib/gen'
	import Label from '$lib/components/Label.svelte'
	import TextInput, {
		inputBaseClass,
		inputBorderClass,
		inputSizeClasses
	} from '$lib/components/text_input/TextInput.svelte'
	import DucklakePicker from '$lib/components/DucklakePicker.svelte'
	import DatatablePicker from '$lib/components/DatatablePicker.svelte'
	import Path from '$lib/components/Path.svelte'
	import Section from '$lib/components/Section.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import DateTimeInput from '$lib/components/DateTimeInput.svelte'
	import ScheduleAdvancedOptions, {
		emptyScheduleAdvanced,
		scheduleAdvancedCfg
	} from '$lib/components/triggers/schedules/ScheduleAdvancedOptions.svelte'
	import { workspaceStore } from '$lib/stores'
	import { ArrowLeft, ChevronDown, CornerDownLeft, Sparkles } from 'lucide-svelte'
	import { tick } from 'svelte'
	import { arrowTabNav } from '$lib/attachments/arrowTabNav'
	import { selectAndAdvanceTo } from '$lib/attachments/selectAndAdvanceTo'

	interface Props {
		kinds: PipelineInsertKind[]
		languages?: Array<{ label: string; lang: SupportedLanguage }>
		pathPrefix?: string
		defaultPathSuffix?: string
		onPick: (pick: PipelineInsertPick) => void
		trigger: import('svelte').Snippet
		placement?: 'bottom' | 'top' | 'left' | 'right'
	}

	let {
		kinds,
		languages = [],
		pathPrefix = '',
		trigger: triggerSnippet,
		placement = 'bottom',
		defaultPathSuffix,
		onPick
	}: Props = $props()

	// When there's only one trigger kind, hide the Trigger column entirely
	// and pre-select it so the user lands directly on the Language picker.
	const singleKind = $derived(kinds.length === 1 ? kinds[0] : undefined)

	const buildEmptySelected = () => ({
		triggerId: singleKind?.id,
		language: undefined as undefined | ScriptLang,
		outputId: undefined as undefined | PipelineOutputKind,
		scriptPath: `${defaultPathSuffix ?? 'pipeline_script'}_${shortSlug()}`,
		aiPrompt: undefined as undefined | string
	})
	let selected = $state(buildEmptySelected())
	let selectedKind = $derived(kinds.find((k) => k.id === selected.triggerId))

	// Steps after the first, each configuring something the picks call for.
	type ConfigStep = 'schedule' | 'asset'
	const NAMED_OUTPUT_KINDS: PipelineOutputKind[] = ['materialize', 'datatable', 'ducklake']
	let configSteps = $derived.by<ConfigStep[]>(() => {
		const steps: ConfigStep[] = []
		if (selected.triggerId === 'schedule') steps.push('schedule')
		if (selected.outputId && NAMED_OUTPUT_KINDS.includes(selected.outputId)) steps.push('asset')
		return steps
	})
	// 0 is the trigger/language/output/path step; `n` is `configSteps[n - 1]`.
	let step = $state(0)
	let currentConfigStep = $derived(step > 0 ? configSteps[step - 1] : undefined)
	let isLastStep = $derived(step >= configSteps.length)

	const buildEmptyConfig = () => ({
		schedule: '0 0 12 * *',
		timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
		cronVersion: 'v2',
		validCron: true,
		// Follows the script path until the Path field is edited.
		schedulePath: '',
		schedulePathDirty: false,
		schedulePathError: '',
		summary: '',
		description: '',
		pauseUntil: false,
		pausedUntil: undefined as string | undefined,
		advanced: emptyScheduleAdvanced(),
		// `store` is the data table or DuckLake catalog name. The table name
		// follows the script name until the user types in it.
		asset: undefined as
			| undefined
			| { kind: 'ducklake' | 'datatable'; store: string | undefined; table: string },
		tableEdited: false
	})
	let config = $state(buildEmptyConfig())

	function resetWizard() {
		selected = buildEmptySelected()
		config = buildEmptyConfig()
		step = 0
	}

	const TABLE_NAME_RE = /^[^/\s]+$/
	let assetValid = $derived(
		!!config.asset?.store && TABLE_NAME_RE.test(config.asset.table.trim())
	)
	let stepValid = $derived(
		currentConfigStep === 'schedule'
			? config.validCron && !!config.schedule.trim() && !config.schedulePathError
			: currentConfigStep === 'asset'
				? assetValid
				: !!selected.scriptPath.trim()
	)

	function enterAssetStep() {
		const kind = selected.outputId === 'datatable' ? 'datatable' : 'ducklake'
		if (config.asset?.kind !== kind) {
			config.asset = { kind, store: 'main', table: '' }
			config.tableEdited = false
		}
		if (!config.tableEdited && config.asset) {
			config.asset.table = selected.scriptPath.trim().split('/').pop() ?? ''
		}
	}

	// Refs to the column containers so Enter inside one column can hand
	// focus off to the first tabbable in the next column.
	let languageEl: HTMLElement | undefined
	let outputEl: HTMLElement | undefined
	// Refs into the bottom panel for the Output → Path → AI Prompt → Save chain. The AI
	// prompt is absent where the workspace hid the assistant, so Path advances to Save.
	let pathEl: HTMLElement | undefined
	let aiPromptEl: HTMLElement | undefined
	let saveEl: HTMLElement | undefined

	let compatibleKinds = $derived.by<PipelineOutputKind[]>(() => {
		if (!selected.language) return []
		return compatibleOutputKinds(selected.language)
	})

	let visibleOutputKinds = $derived(
		PIPELINE_OUTPUT_KINDS.filter((k) => compatibleKinds.includes(k.id))
	)

	let showBottomPanel = $derived(selected.triggerId && selected.language && selected.outputId)

	function shortSlug(len = 4): string {
		const a = 'abcdefghijklmnopqrstuvwxyz0123456789'
		let out = ''
		for (let i = 0; i < len; i++) out += a[Math.floor(Math.random() * a.length)]
		return out
	}

	// A step change re-renders the popover body, dropping focus to <body>, where
	// keystrokes (Enter, Cmd+Enter) no longer reach the step's key handlers.
	let contentEl: HTMLElement | undefined = $state()
	async function focusCurrentStep() {
		await tick()
		const target =
			step === 0
				? pathEl?.querySelector('input')
				: contentEl?.querySelector<HTMLElement>('[data-autofocus] input')
		target?.focus()
	}

	function goBack() {
		step -= 1
		void focusCurrentStep()
	}

	function confirm(close: () => void) {
		const suffix = selected.scriptPath.trim()
		if (!suffix || !selected.triggerId || !selected.language || !selected.outputId) return
		if (!stepValid) return
		if (!isLastStep) {
			step += 1
			if (configSteps[step - 1] === 'asset') enterAssetStep()
			if (configSteps[step - 1] === 'schedule' && !config.schedulePathDirty) {
				config.schedulePath = `${pathPrefix}${suffix}_schedule`
			}
			void focusCurrentStep()
			return
		}
		const trimmedPrompt = selected.aiPrompt?.trim()
		onPick({
			kindId: selected.triggerId,
			language: selected.language,
			path: pathPrefix + suffix,
			outputKind: selected.outputId,
			aiPrompt: trimmedPrompt && trimmedPrompt.length > 0 ? trimmedPrompt : undefined,
			schedule: configSteps.includes('schedule')
				? {
						path: config.schedulePath,
						schedule: config.schedule.trim(),
						timezone: config.timezone,
						cron_version: config.cronVersion,
						summary: config.summary.trim() || undefined,
						description: config.description,
						paused_until: config.pauseUntil ? config.pausedUntil : undefined,
						is_flow: false,
						args: {},
						...scheduleAdvancedCfg($state.snapshot(config.advanced))
					}
				: undefined,
			outputAsset:
				configSteps.includes('asset') && config.asset
					? {
							kind: config.asset.kind as AssetKind,
							path: `${config.asset.store}/${config.asset.table.trim()}`
						}
					: undefined
		})
		close()
	}
</script>

<Popover
	enableFlyTransition
	contentClasses={twMerge(
		'p-0 bg-surface overflow-hidden relative transition-height',
		// The first step's columns (w-56 + w-48 + w-80), held across steps.
		singleKind ? 'w-[32rem]' : 'w-[46rem]',
		currentConfigStep === 'schedule'
			? 'h-[27rem]'
			: currentConfigStep === 'asset'
				? 'h-[14rem]'
				: showBottomPanel
					? 'h-[26rem]'
					: 'h-[22rem]'
	)}
	class="inline-block"
	usePointerDownOutside
	floatingConfig={{
		placement,
		strategy: 'absolute',
		gutter: 8,
		overflowPadding: 16,
		flip: true,
		fitViewport: true,
		overlap: false
	}}
	onClose={resetWizard}
>
	{#snippet trigger()}
		{@render triggerSnippet?.()}
	{/snippet}
	{#snippet content({ close })}
		<div class="h-full" bind:this={contentEl}>
		{#if currentConfigStep}
			{@render configStepSection(currentConfigStep, close)}
		{:else}
			<div class="flex flex-col h-full">
				<div class={'flex flex-row transition-height divide-x overflow-y-scroll'}>
					{@render topSection()}
				</div>
				<div
					class={twMerge(
						'flex flex-col gap-5 grow transition-height px-4 border-t',
						showBottomPanel ? 'h-[14rem] py-4' : 'h-0'
					)}
				>
					{@render bottomSection(close)}
				</div>
			</div>
		{/if}
		</div>
	{/snippet}
</Popover>

{#snippet topSection()}
	{#if !singleKind}
		<div
			class={twMerge('flex flex-col gap-1 p-2 w-56 shrink-0 overflow-auto')}
			{@attach arrowTabNav({ onKeyDown: selectAndAdvanceTo(() => languageEl) })}
		>
			<div class="text-2xs font-normal text-secondary ml-2 mb-1">Trigger</div>
			{#each kinds as k}
				{@const isSelected = selected.triggerId == k.id}
				<Button
					variant="subtle"
					btnClasses={'text-left'}
					onClick={() => (selected.triggerId = k.id)}
					selected={isSelected}
				>
					{#if k.icon}
						{@const Icon = k.icon}
						<Icon
							size={14}
							class={twMerge(
								'shrink-0 my-auto mr-1.5',
								isSelected ? 'text-accent' : 'text-secondary'
							)}
						/>
					{/if}
					<span class="flex flex-col items-start flex-1 min-w-0">
						<span class="text-xs font-normal leading-tight">{k.label}</span>
						{#if k.description}
							<span
								class={twMerge(
									'text-2xs font-normal leading-snug mt-0.5',
									isSelected ? 'text-accent/80' : 'text-hint'
								)}
							>
								{k.description}
							</span>
						{/if}
					</span>
				</Button>
			{/each}
		</div>
	{/if}

	<div
		bind:this={languageEl}
		class={twMerge(
			'flex flex-col gap-1 p-2 overflow-auto transition-opacity w-48',
			selected.triggerId ? '' : 'opacity-20'
		)}
		{@attach arrowTabNav({ onKeyDown: selectAndAdvanceTo(() => outputEl) })}
	>
		<div class="text-2xs font-normal text-secondary ml-2 mb-1">Language</div>
		{#each languages as l}
			{@const isSelected = selected.language === l.lang}
			<Button
				variant="subtle"
				unifiedSize="sm"
				btnClasses="justify-start"
				selected={isSelected}
				onClick={() => {
					selected.language = l.lang
					const _compatibleOutputKinds = compatibleOutputKinds(l.lang)
					if (selected.outputId && !_compatibleOutputKinds.includes(selected.outputId)) {
						selected.outputId = _compatibleOutputKinds[0]
					}
				}}
			>
				<LanguageIcon lang={l.lang} width={14} height={14} />
				<span class="grow truncate text-left text-xs">{l.label}</span>
			</Button>
		{/each}
	</div>

	<div
		bind:this={outputEl}
		class={twMerge(
			'flex flex-col gap-1 p-2 grow w-80 overflow-auto transition-opacity',
			selected.triggerId && selected.language ? '' : 'opacity-20'
		)}
		{@attach arrowTabNav({ onKeyDown: selectAndAdvanceTo(() => pathEl, { timeout: 50 }) })}
	>
		<div class="text-2xs font-normal text-secondary ml-2 mb-1">Output asset</div>
		{#each visibleOutputKinds.length ? visibleOutputKinds : PIPELINE_OUTPUT_KINDS as k}
			{@const isSelected = selected.outputId === k.id}
			<Button variant="subtle" selected={isSelected} onClick={() => (selected.outputId = k.id)}>
				<span class="flex flex-col items-start flex-1 min-w-0 text-left">
					<span class="text-xs font-normal leading-tight">{k.label}</span>
					{#if k.description}
						<span
							class={twMerge(
								'text-2xs font-normal leading-snug mt-0.5',
								isSelected ? 'text-accent/80' : 'text-hint'
							)}
						>
							{k.description}
						</span>
					{/if}
				</span>
			</Button>
		{/each}
	</div>
{/snippet}

{#snippet bottomSection(close: () => void)}
	<Label label="Path">
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div bind:this={pathEl} class="flex" onkeydown={selectAndAdvanceTo(() => aiPromptEl ?? saveEl)}>
			<div
				class="border rounded-md rounded-r-none border-r-0 text-xs w-fit shrink-0 whitespace-nowrap flex items-center px-2 text-secondary bg-surface-input"
			>
				{pathPrefix}
			</div>
			<TextInput bind:value={selected.scriptPath} class="rounded-l-none" />
		</div>
	</Label>
	{#if !$copilotInfo.workspaceDisabled}
		<Label label="AI Prompt (optional)">
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				bind:this={aiPromptEl}
				onkeydown={(e) =>
					(e.metaKey || e.ctrlKey) && e.key === 'Enter' && (confirm(close), e.stopPropagation())}
			>
				<TextInput
					class="resize-none h-12 !max-h-12"
					underlyingInputEl="textarea"
					inputProps={{
						placeholder: 'Describe what the script should do — leave empty to use a template'
					}}
					bind:value={selected.aiPrompt}
				/>
			</div>
		</Label>
	{/if}
	<div class="ml-auto" bind:this={saveEl}>
		{@render confirmButton(close)}
	</div>
{/snippet}

{#snippet confirmButton(close: () => void)}
	{@const hasAiPrompt = !!selected.aiPrompt?.trim()}
	<Button
		variant="accent"
		btnClasses="w-fit"
		disabled={!stepValid}
		onClick={() => confirm(close)}
		startIcon={isLastStep && hasAiPrompt ? { icon: Sparkles } : undefined}
		shortCut={{ Icon: CornerDownLeft }}
		>{!isLastStep
			? step === 0
				? 'Configure'
				: 'Next'
			: hasAiPrompt
				? 'Generate'
				: selectedKind?.configuredAfterCreate
					? 'Configure'
					: 'Create'}</Button
	>
{/snippet}

{#snippet configStepSection(configStep: ConfigStep, close: () => void)}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="flex flex-col h-full"
		onkeydown={(e) => {
			if (e.key !== 'Enter') return
			const target = e.target as HTMLElement | null
			// Cmd/Ctrl+Enter confirms from anywhere; bare Enter only from the step's own
			// inputs, not from the Advanced fields (a textarea, pickers with their own Enter).
			const bareEnterSubmits = target?.tagName === 'INPUT' && !target.closest('[data-advanced]')
			if (e.metaKey || e.ctrlKey || bareEnterSubmits) {
				e.preventDefault()
				e.stopPropagation()
				confirm(close)
			}
		}}
	>
		<div class="flex items-center gap-2 px-2 py-2 border-b">
			<Button
				variant="subtle"
				unifiedSize="sm"
				iconOnly
				startIcon={{ icon: ArrowLeft }}
				onClick={goBack}
				title="Back"
			/>
			<span class="text-xs font-semibold text-emphasis">
				{configStep === 'schedule' ? 'Schedule' : 'Output asset'}
			</span>
			<span class="text-2xs text-hint ml-auto pr-2"
				>Step {step + 1} of {configSteps.length + 1}</span
			>
		</div>
		<div class="flex flex-col gap-2 grow overflow-auto p-4">
			{#if configStep === 'schedule'}
				<span class="text-2xs text-secondary">
					Saved as a draft schedule and deployed along with the pipeline.
				</span>
				<div data-autofocus class="contents">
				<CronInput
					bind:schedule={config.schedule}
					bind:timezone={config.timezone}
					bind:validCRON={config.validCron}
					bind:cronVersion={config.cronVersion}
				/>
				</div>
				<div data-advanced class="mt-4">
					<Section label="Advanced" collapsable initiallyCollapsed>
						<div class="flex flex-col gap-6">
							<Label label="Summary">
								<TextInput
									bind:value={config.summary}
									inputProps={{ placeholder: 'Short summary to be displayed when listed' }}
								/>
							</Label>
							<Label label="Path">
								<Path
									bind:path={config.schedulePath}
									bind:dirty={config.schedulePathDirty}
									bind:error={config.schedulePathError}
									initialPath=""
									checkInitialPathExistence
									namePlaceholder="schedule"
									kind="schedule"
								/>
							</Label>
							<Label label="Description">
								<TextInput
									underlyingInputEl="textarea"
									bind:value={config.description}
									inputProps={{ placeholder: 'What this schedule does and how to use it' }}
								/>
							</Label>
							<div class="flex flex-col gap-1">
								<Toggle
									options={{
										right: 'Pause schedule until...',
										rightTooltip:
											'Pausing the schedule will program the next job to run as if the schedule starts at the time the pause is lifted, instead of now.'
									}}
									bind:checked={config.pauseUntil}
								/>
								{#if config.pauseUntil}
									<DateTimeInput bind:value={config.pausedUntil} />
								{/if}
							</div>
							<ScheduleAdvancedOptions
								wsId={$workspaceStore}
								itemKind="script"
								canWrite
								bind:errorHandlerSelected={config.advanced.errorHandlerSelected}
								bind:errorHandlerPath={config.advanced.errorHandlerPath}
								bind:errorHandleritemKind={config.advanced.errorHandleritemKind}
								bind:errorHandlerExtraArgs={config.advanced.errorHandlerExtraArgs}
								bind:wsErrorHandlerMuted={config.advanced.wsErrorHandlerMuted}
								bind:failedTimes={config.advanced.failedTimes}
								bind:failedExact={config.advanced.failedExact}
								bind:recoveryHandlerSelected={config.advanced.recoveryHandlerSelected}
								bind:recoveryHandlerPath={config.advanced.recoveryHandlerPath}
								bind:recoveryHandlerItemKind={config.advanced.recoveryHandlerItemKind}
								bind:recoveryHandlerExtraArgs={config.advanced.recoveryHandlerExtraArgs}
								bind:recoveredTimes={config.advanced.recoveredTimes}
								bind:successHandlerSelected={config.advanced.successHandlerSelected}
								bind:successHandlerPath={config.advanced.successHandlerPath}
								bind:successHandlerItemKind={config.advanced.successHandlerItemKind}
								bind:successHandlerExtraArgs={config.advanced.successHandlerExtraArgs}
								bind:retry={config.advanced.retry}
								bind:dynamicSkipPath={config.advanced.dynamicSkipPath}
								bind:tag={config.advanced.tag}
							/>
						</div>
					</Section>
				</div>
			{:else if configStep === 'asset' && config.asset}
				<Label label="Name">
					<div
						class={twMerge(
							inputBaseClass,
							inputBorderClass({ error: !assetValid }),
							inputSizeClasses.md,
							'flex flex-row items-center gap-0 py-0 pl-2 pr-0'
						)}
					>
						<span class="text-xs text-secondary shrink-0">{config.asset.kind}://</span>
						{#if config.asset.kind === 'datatable'}
							<DatatablePicker
								bind:value={config.asset.store}
								class="shrink-0"
								selectInputClass="!border-none"
								placeholder="data table"
								useContentEditable
								RightIcon={ChevronDown}
							/>
						{:else}
							<DucklakePicker
								bind:value={config.asset.store}
								class="shrink-0"
								selectInputClass="!border-none"
								placeholder="catalog"
								useContentEditable
								RightIcon={ChevronDown}
							/>
						{/if}
						<span class="text-sm text-secondary">/</span>
						<div data-autofocus class="grow flex">
							<TextInput
								bind:value={config.asset.table}
								class="!border-none grow"
								inputProps={{
									placeholder: 'table',
									oninput: () => (config.tableEdited = true)
								}}
							/>
						</div>
					</div>
				</Label>
				<span class="text-2xs text-hint">
					{config.asset.kind === 'datatable' ? 'Data table' : 'DuckLake catalog'}, then the table
					name
				</span>
			{/if}
		</div>
		<div class="flex px-4 py-3 border-t">
			<div class="ml-auto">
				{@render confirmButton(close)}
			</div>
		</div>
	</div>
{/snippet}
