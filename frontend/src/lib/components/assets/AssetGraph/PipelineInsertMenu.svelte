<script lang="ts" module>
	import type { ComponentType } from 'svelte'
	import type { SupportedLanguage } from '$lib/common'

	export type PipelineInsertKind = {
		// Machine-readable id; drives which right-column panel renders.
		id: string
		label: string
		description?: string
		icon?: ComponentType
		// When pickLanguage is true, the right panel shows the language
		// picker (and after a language is chosen, a path-entry stage).
		// Otherwise onSelect is called directly with no language / path.
		pickLanguage?: boolean
	}

	export type PipelineInsertPick = {
		kindId: string
		language?: SupportedLanguage
		path?: string
		// Picked output asset kind. Optional because some menu instances
		// (those without `pickOutputKind`) skip that stage entirely.
		outputKind?: string
	}
</script>

<script lang="ts">
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import LanguageIcon from '$lib/components/common/languageIcons/LanguageIcon.svelte'
	import { ArrowLeft, ChevronRight } from 'lucide-svelte'
	import { tick } from 'svelte'
	import {
		PIPELINE_OUTPUT_KINDS,
		compatibleOutputKinds,
		type PipelineOutputKind
	} from './pipelineTemplates'
	import type { ScriptLang } from '$lib/gen'

	interface Props {
		kinds: PipelineInsertKind[]
		languages?: Array<{ label: string; lang: SupportedLanguage }>
		// Non-editable path prefix shown as a read-only chip before the
		// user-editable suffix (e.g. `f/<folder>/`). The final path passed
		// in onPick is `pathPrefix + suffix`.
		pathPrefix?: string
		// Default suffix seeded into the editable input when the user
		// reaches the path stage (e.g. `new_pipeline_script`).
		defaultPathSuffix?: string
		// When true, after the user picks a language we add an output-kind
		// stage between language and path. The picked kind is forwarded in
		// onPick(pick.outputKind). When false (or omitted), the menu jumps
		// directly from language → path, matching the legacy two-stage flow.
		pickOutputKind?: boolean
		onPick: (pick: PipelineInsertPick) => void
		trigger: import('svelte').Snippet
		placement?: 'bottom' | 'top' | 'left' | 'right'
	}

	let {
		kinds,
		languages = [],
		pathPrefix = '',
		defaultPathSuffix = '',
		pickOutputKind = false,
		onPick,
		trigger: triggerSnippet,
		placement = 'bottom'
	}: Props = $props()

	// Flow stages: kind → lang → (output) → path → confirm. `stage` drives
	// the right column. Only the `language` kinds reach the lang/path
	// stages. The `output` stage is gated behind `pickOutputKind` so menus
	// that don't need it (legacy two-column callers) keep their old flow.
	let selectedKindId = $state<string>(kinds[0]?.id ?? '')
	let selectedKind = $derived(kinds.find((k) => k.id === selectedKindId) ?? kinds[0])
	let stage = $state<'lang' | 'output' | 'path' | 'description'>(
		kinds[0]?.pickLanguage ? 'lang' : 'description'
	)
	let selectedLanguage = $state<SupportedLanguage | undefined>(undefined)
	let selectedOutputKind = $state<PipelineOutputKind | undefined>(undefined)
	let pathSuffix = $state('')
	let pathInput: HTMLInputElement | undefined = $state(undefined)

	// Output kinds that have a real template for the picked language. We
	// hide non-compatible kinds entirely rather than greying them — keeps
	// the picker scannable and the user never lands on a kind that would
	// silently fall back to the generic body.
	let compatibleKinds = $derived.by<PipelineOutputKind[]>(() => {
		if (!selectedLanguage) return []
		return compatibleOutputKinds(selectedLanguage as ScriptLang)
	})

	// Popover closing doesn't unmount its content, so state persists across
	// opens. Reset everything back to initial state on close so the next
	// open starts fresh — otherwise reopening lands on the previous path
	// stage with the previous suffix, and a subsequent Enter creates a
	// duplicate of the first draft.
	function resetMenuState() {
		selectedKindId = kinds[0]?.id ?? ''
		stage = kinds[0]?.pickLanguage ? 'lang' : 'description'
		selectedLanguage = undefined
		selectedOutputKind = undefined
		pathSuffix = ''
	}

	function handleKindClick(k: PipelineInsertKind, close: () => void) {
		if (!k.pickLanguage) {
			onPick({ kindId: k.id })
			close()
			return
		}
		selectedKindId = k.id
		stage = 'lang'
		selectedLanguage = undefined
		selectedOutputKind = undefined
		pathSuffix = ''
	}

	// Short random slug appended to the default suffix so that opening the
	// menu twice in a row seeds two distinct paths — otherwise creating
	// multiple pipeline scripts in the same folder collides on the same
	// `f/<folder>/<suffix>` and they all become revisions of one script.
	function shortSlug(len = 4): string {
		const a = 'abcdefghijklmnopqrstuvwxyz0123456789'
		let out = ''
		for (let i = 0; i < len; i++) out += a[Math.floor(Math.random() * a.length)]
		return out
	}

	async function focusPathInput() {
		await tick()
		pathInput?.focus()
		pathInput?.select()
	}

	async function handleLanguageClick(lang: SupportedLanguage) {
		selectedLanguage = lang
		// If this menu wants an output-kind stage, route through it; the
		// path suffix only gets seeded once the user has confirmed both
		// language and output kind.
		if (pickOutputKind) {
			selectedOutputKind = undefined
			stage = 'output'
			return
		}
		const base = defaultPathSuffix || 'pipeline_script'
		pathSuffix = `${base}_${shortSlug()}`
		stage = 'path'
		await focusPathInput()
	}

	async function handleOutputKindClick(kind: PipelineOutputKind) {
		selectedOutputKind = kind
		const base = defaultPathSuffix || 'pipeline_script'
		pathSuffix = `${base}_${shortSlug()}`
		stage = 'path'
		await focusPathInput()
	}

	function confirmPath(close: () => void) {
		const suffix = pathSuffix.trim()
		if (!suffix || !selectedLanguage) return
		if (pickOutputKind && !selectedOutputKind) return
		onPick({
			kindId: selectedKindId,
			language: selectedLanguage,
			path: pathPrefix + suffix,
			outputKind: pickOutputKind ? selectedOutputKind : undefined
		})
		close()
	}

	function handlePathKeydown(e: KeyboardEvent, close: () => void) {
		if (e.key === 'Enter') {
			e.preventDefault()
			confirmPath(close)
		} else if (e.key === 'Escape') {
			e.preventDefault()
			stage = pickOutputKind ? 'output' : 'lang'
		}
	}
</script>

<Popover
	contentClasses="p-0 bg-surface overflow-hidden"
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
	on:openChange={(e) => {
		if (!e.detail) resetMenuState()
	}}
>
	{#snippet trigger()}
		{@render triggerSnippet?.()}
	{/snippet}
	{#snippet content({ close })}
		{@const singleKind = kinds.length === 1}
		{@const widthClass = pickOutputKind
			? singleKind
				? 'w-[520px]'
				: 'w-[720px]'
			: singleKind
				? 'w-[360px]'
				: 'w-[560px]'}
		<div
			class={[
				'flex flex-row bg-surface-tertiary h-[280px]',
				singleKind ? '' : 'divide-x',
				widthClass
			].join(' ')}
		>
			<!-- Left column: kind picker. Only shown when there's more than one
			     option — single-kind menus jump straight to language/path. -->
			{#if !singleKind}
				<div class="flex flex-col gap-1 p-2 w-52 shrink-0 overflow-auto">
					{#each kinds as k}
						{@const isSelected = selectedKindId === k.id}
						<button
							type="button"
							onclick={() => handleKindClick(k, close)}
							class={[
								'flex items-start gap-2 px-2 py-2 rounded-md text-left transition-colors',
								isSelected
									? 'bg-surface-selected text-emphasis'
									: 'hover:bg-surface-hover text-primary'
							].join(' ')}
						>
							{#if k.icon}
								{@const Icon = k.icon}
								<Icon size={14} class="shrink-0 mt-0.5 text-secondary" />
							{/if}
							<span class="flex flex-col items-start flex-1 min-w-0">
								<span class="text-sm font-medium leading-tight">{k.label}</span>
								{#if k.description}
									<span class="text-2xs text-tertiary font-normal leading-snug mt-0.5">
										{k.description}
									</span>
								{/if}
							</span>
							{#if k.pickLanguage}
								<ChevronRight size={12} class="shrink-0 mt-1 text-secondary" />
							{/if}
						</button>
					{/each}
				</div>
			{/if}

			<!-- Right column: stage-driven -->
			<div class="flex flex-col grow min-w-0 overflow-hidden">
				{#if stage === 'lang'}
					<div class="flex flex-col gap-1 p-2 grow overflow-auto">
						<div class="text-2xs font-normal text-secondary ml-2 mb-1">Language</div>
						{#each languages as l}
							<Button
								variant="subtle"
								unifiedSize="sm"
								btnClasses="justify-start"
								onClick={() => handleLanguageClick(l.lang)}
							>
								<LanguageIcon lang={l.lang} width={14} height={14} />
								<span class="grow truncate text-left text-sm">{l.label}</span>
							</Button>
						{/each}
					</div>
				{:else if stage === 'output' && selectedLanguage}
					<div class="flex flex-col gap-1 p-2 grow overflow-auto">
						<div class="flex items-center gap-2 mb-1">
							<Button
								variant="subtle"
								unifiedSize="xs"
								startIcon={{ icon: ArrowLeft }}
								iconOnly
								title="Back to language"
								onClick={() => (stage = 'lang')}
							/>
							<div class="flex items-center gap-1.5">
								<LanguageIcon lang={selectedLanguage} width={13} height={13} />
								<span class="text-xs font-medium">{selectedLanguage}</span>
							</div>
						</div>
						<div class="text-2xs font-normal text-secondary ml-2 mb-1">Output asset</div>
						{#each PIPELINE_OUTPUT_KINDS.filter((k) => compatibleKinds.includes(k.id)) as k}
							<button
								type="button"
								onclick={() => handleOutputKindClick(k.id)}
								class="flex flex-col items-start gap-0.5 px-2 py-2 rounded-md text-left transition-colors hover:bg-surface-hover"
							>
								<span class="text-sm font-medium leading-tight">{k.label}</span>
								<span class="text-2xs text-tertiary font-normal leading-snug">
									{k.description}
								</span>
							</button>
						{/each}
						{#if compatibleKinds.length === 0}
							<span class="text-2xs text-tertiary px-2">No output presets for this language.</span>
						{/if}
					</div>
				{:else if stage === 'path' && selectedLanguage}
					{@const outputMeta = selectedOutputKind
						? PIPELINE_OUTPUT_KINDS.find((k) => k.id === selectedOutputKind)
						: undefined}
					<div class="flex flex-col gap-3 p-3 grow overflow-auto">
						<div class="flex items-center gap-2">
							<Button
								variant="subtle"
								unifiedSize="xs"
								startIcon={{ icon: ArrowLeft }}
								iconOnly
								title={pickOutputKind ? 'Back to output' : 'Back to language'}
								onClick={() => (stage = pickOutputKind ? 'output' : 'lang')}
							/>
							<div class="flex items-center gap-1.5">
								<LanguageIcon lang={selectedLanguage} width={13} height={13} />
								<span class="text-xs font-medium">{selectedLanguage}</span>
							</div>
							{#if outputMeta}
								<span class="text-2xs text-tertiary">·</span>
								<span class="text-2xs text-secondary">{outputMeta.label}</span>
							{/if}
						</div>
						<div class="flex flex-col gap-1">
							<span class="text-2xs font-normal text-secondary">Path</span>
							<!-- Prefix is a separate non-editable <span> sitting next to
							     the <input>, so users physically can't delete it with
							     backspace. The final path is pathPrefix + pathSuffix. -->
							<div
								class="flex items-stretch border rounded-md bg-surface overflow-hidden focus-within:ring-2 focus-within:ring-emerald-400"
							>
								{#if pathPrefix}
									<span
										class="flex items-center px-2 bg-surface-secondary text-tertiary text-sm font-mono border-r select-none"
										title="Folder-scoped prefix (fixed)"
									>
										{pathPrefix}
									</span>
								{/if}
								<input
									bind:this={pathInput}
									bind:value={pathSuffix}
									onkeydown={(e) => handlePathKeydown(e, close)}
									class="flex-1 min-w-0 px-2 py-1.5 text-sm font-mono bg-transparent focus:outline-none"
									placeholder="my_script"
								/>
							</div>
							<span class="text-2xs text-tertiary">Press Enter to create</span>
						</div>
						<div class="flex justify-end mt-auto">
							<Button
								variant="accent"
								unifiedSize="sm"
								disabled={!pathSuffix.trim()}
								onClick={() => confirmPath(close)}
							>
								Create
							</Button>
						</div>
					</div>
				{:else if selectedKind?.description}
					<div class="flex-1 flex items-center justify-center p-4 text-center">
						<span class="text-xs text-secondary">{selectedKind.description}</span>
					</div>
				{/if}
			</div>
		</div>
	{/snippet}
</Popover>
