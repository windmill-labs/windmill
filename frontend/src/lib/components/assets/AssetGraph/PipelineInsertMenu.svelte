<script lang="ts" module>
	import type { ComponentType } from 'svelte'
	import type { SupportedLanguage } from '$lib/common'

	export type PipelineInsertKind = {
		// Machine-readable id; drives which right-column panel renders.
		id: string
		label: string
		description?: string
		icon?: ComponentType
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
		// Set when the user picked an ingestion template instead of the
		// trigger/language/output columns (which the template implies).
		// `kindId` is 'ingestion_template' in that case and `language` /
		// `outputKind` are unset.
		templateId?: string
	}
</script>

<script lang="ts">
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import LanguageIcon from '$lib/components/common/languageIcons/LanguageIcon.svelte'
	import { twMerge } from 'tailwind-merge'
	import {
		compatibleOutputKinds,
		PIPELINE_OUTPUT_KINDS,
		type PipelineOutputKind
	} from './pipelineTemplates'
	import type { ScriptLang } from '$lib/gen'
	import Label from '$lib/components/Label.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { CornerDownLeft, Sparkles } from 'lucide-svelte'
	import { arrowTabNav } from '$lib/attachments/arrowTabNav'
	import { selectAndAdvanceTo } from '$lib/attachments/selectAndAdvanceTo'

	interface Props {
		kinds: PipelineInsertKind[]
		// Ingestion-template entries rendered below the trigger kinds. A
		// template pick short-circuits the language/output columns (the
		// template carries both) straight to the path panel.
		templates?: PipelineInsertKind[]
		languages?: Array<{ label: string; lang: SupportedLanguage }>
		pathPrefix?: string
		defaultPathSuffix?: string
		onPick: (pick: PipelineInsertPick) => void
		trigger: import('svelte').Snippet
		placement?: 'bottom' | 'top' | 'left' | 'right'
	}

	let {
		kinds,
		templates = [],
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
		templateId: undefined as undefined | string,
		language: undefined as undefined | ScriptLang,
		outputId: undefined as undefined | PipelineOutputKind,
		scriptPath: `${defaultPathSuffix ?? 'pipeline_script'}_${shortSlug()}`,
		aiPrompt: undefined as undefined | string
	})
	let selected = $state(buildEmptySelected())

	// Refs to the column containers so Enter inside one column can hand
	// focus off to the first tabbable in the next column.
	let languageEl: HTMLElement | undefined
	let outputEl: HTMLElement | undefined
	// Refs into the bottom panel for the Output → Path → AI Prompt → Save chain.
	let pathEl: HTMLElement | undefined
	let aiPromptEl: HTMLElement | undefined

	let compatibleKinds = $derived.by<PipelineOutputKind[]>(() => {
		if (!selected.language) return []
		return compatibleOutputKinds(selected.language)
	})

	let visibleOutputKinds = $derived(
		PIPELINE_OUTPUT_KINDS.filter((k) => compatibleKinds.includes(k.id))
	)

	let showBottomPanel = $derived(
		!!selected.templateId || (selected.triggerId && selected.language && selected.outputId)
	)

	function shortSlug(len = 4): string {
		const a = 'abcdefghijklmnopqrstuvwxyz0123456789'
		let out = ''
		for (let i = 0; i < len; i++) out += a[Math.floor(Math.random() * a.length)]
		return out
	}

	function confirm(close: () => void) {
		const suffix = selected.scriptPath.trim()
		if (!suffix) return
		if (selected.templateId) {
			onPick({
				kindId: 'ingestion_template',
				templateId: selected.templateId,
				path: pathPrefix + suffix
			})
			close()
			return
		}
		if (!selected.triggerId || !selected.language || !selected.outputId) return
		const trimmedPrompt = selected.aiPrompt?.trim()
		onPick({
			kindId: selected.triggerId,
			language: selected.language,
			path: pathPrefix + suffix,
			outputKind: selected.outputId,
			aiPrompt: trimmedPrompt && trimmedPrompt.length > 0 ? trimmedPrompt : undefined
		})
		close()
	}
</script>

<Popover
	enableFlyTransition
	contentClasses={twMerge(
		'p-0 bg-surface overflow-hidden relative transition-height',
		showBottomPanel ? 'h-[26rem]' : 'h-[22rem]'
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
	onClose={() => (selected = buildEmptySelected())}
>
	{#snippet trigger()}
		{@render triggerSnippet?.()}
	{/snippet}
	{#snippet content({ close })}
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
	{/snippet}
</Popover>

{#snippet topSection()}
	{#if !singleKind}
		<div
			class={twMerge('flex flex-col gap-1 p-2 w-56 shrink-0 overflow-auto')}
			{@attach arrowTabNav({ onKeyDown: selectAndAdvanceTo(() => languageEl) })}
		>
			<!-- Templates lead the column: the trigger list below is taller than
			     the popover viewport, so anything after it sits below the fold
			     and is effectively invisible. -->
			{#if templates.length > 0}
				<div class="text-2xs font-normal text-secondary ml-2 mb-1">Ingestion templates</div>
				{#each templates as t}
					{@const isSelected = selected.templateId === t.id}
					<Button
						variant="subtle"
						btnClasses={'text-left'}
						onClick={() => {
							// A template implies trigger + language + output — selecting
							// one short-circuits the other columns straight to the path
							// panel (see `showBottomPanel`).
							selected.templateId = t.id
							selected.triggerId = undefined
						}}
						selected={isSelected}
					>
						{#if t.icon}
							{@const Icon = t.icon}
							<Icon
								size={14}
								class={twMerge(
									'shrink-0 my-auto mr-1.5',
									isSelected ? 'text-accent' : 'text-secondary'
								)}
							/>
						{/if}
						<span class="flex flex-col items-start flex-1 min-w-0">
							<span class="text-xs font-normal leading-tight">{t.label}</span>
							{#if t.description}
								<span
									class={twMerge(
										'text-2xs font-normal leading-snug mt-0.5',
										isSelected ? 'text-accent/80' : 'text-hint'
									)}
								>
									{t.description}
								</span>
							{/if}
						</span>
					</Button>
				{/each}
			{/if}
			<div
				class={twMerge(
					'text-2xs font-normal text-secondary ml-2 mb-1',
					templates.length > 0 ? 'mt-3 pt-2 border-t' : ''
				)}>Trigger</div
			>
			{#each kinds as k}
				{@const isSelected = selected.triggerId == k.id && !selected.templateId}
				<Button
					variant="subtle"
					btnClasses={'text-left'}
					onClick={() => {
						selected.triggerId = k.id
						selected.templateId = undefined
					}}
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
			selected.triggerId && !selected.templateId ? '' : 'opacity-20'
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
					// Leaving template mode: a language pick means the user is
					// back on the column-by-column flow (symmetric with the
					// trigger buttons above).
					selected.templateId = undefined
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
			selected.triggerId && selected.language && !selected.templateId ? '' : 'opacity-20'
		)}
		{@attach arrowTabNav({ onKeyDown: selectAndAdvanceTo(() => pathEl, { timeout: 50 }) })}
	>
		<div class="text-2xs font-normal text-secondary ml-2 mb-1">Output asset</div>
		{#each visibleOutputKinds.length ? visibleOutputKinds : PIPELINE_OUTPUT_KINDS as k}
			{@const isSelected = selected.outputId === k.id}
			<Button
				variant="subtle"
				selected={isSelected}
				onClick={() => {
					selected.templateId = undefined
					selected.outputId = k.id
				}}
			>
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
		<div bind:this={pathEl} class="flex" onkeydown={selectAndAdvanceTo(() => aiPromptEl)}>
			<div
				class="border rounded-md rounded-r-none border-r-0 text-xs w-fit shrink-0 whitespace-nowrap flex items-center px-2 text-secondary bg-surface-input"
			>
				{pathPrefix}
			</div>
			<TextInput bind:value={selected.scriptPath} class="rounded-l-none" />
		</div>
	</Label>
	{#if !selected.templateId}
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
	{@const hasAiPrompt = !selected.templateId && !!selected.aiPrompt?.trim()}
	<div class="ml-auto">
		<Button
			variant="accent"
			btnClasses="w-fit"
			disabled={!selected.scriptPath.trim()}
			onClick={() => confirm(close)}
			startIcon={hasAiPrompt ? { icon: Sparkles } : undefined}
			shortCut={{ Icon: CornerDownLeft }}>{hasAiPrompt ? 'Generate' : 'Create'}</Button
		>
	</div>
{/snippet}
