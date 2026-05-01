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
		// When true, the language stage shows a second sub-column with
		// compatible output-asset kinds; the user picks both side-by-side
		// and the chosen kind is forwarded as onPick(pick.outputKind).
		// When false (or omitted), language click goes straight to path.
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

	// Flow stages: kind → lang(+output) → path → confirm. `stage` drives
	// the right column. Only the `language` kinds reach the lang/path
	// stages. When `pickOutputKind` is true the lang stage renders two
	// sub-columns (languages | compatible output kinds) so picking happens
	// in one step instead of two sequential screens.
	// Default the lang column to duckdb when present — most pipeline
	// authors want SQL-on-data, and preselecting populates the output
	// column immediately so the user sees what's available without an
	// extra click.
	function defaultLangIdx(): number {
		const i = languages.findIndex((l) => l.lang === 'duckdb')
		return i >= 0 ? i : 0
	}

	let selectedKindId = $state<string>(kinds[0]?.id ?? '')
	let selectedKind = $derived(kinds.find((k) => k.id === selectedKindId) ?? kinds[0])
	let stage = $state<'lang' | 'path' | 'description'>(
		kinds[0]?.pickLanguage ? 'lang' : 'description'
	)
	let selectedLanguage = $state<SupportedLanguage | undefined>(languages[defaultLangIdx()]?.lang)
	let selectedOutputKind = $state<PipelineOutputKind | undefined>(undefined)
	let pathSuffix = $state('')
	let pathInput: HTMLInputElement | undefined = $state(undefined)

	// Keyboard navigation state. `focusCol` tracks which column the arrow
	// keys steer; per-column indexes survive column switches so the user's
	// position is preserved as they move left/right.
	let focusCol = $state<'kind' | 'lang' | 'output'>('lang')
	let kindIdx = $state(0)
	let langIdx = $state(defaultLangIdx())
	let outputIdx = $state(0)
	// Tracks popover open state so the window-level keydown handler can
	// no-op when the menu is closed (the snippet content is reused across
	// opens, so we can't rely on mount/unmount).
	let isOpen = $state(false)

	// Output kinds that have a real template for the picked language. We
	// hide non-compatible kinds entirely rather than greying them — keeps
	// the picker scannable and the user never lands on a kind that would
	// silently fall back to the generic body.
	let compatibleKinds = $derived.by<PipelineOutputKind[]>(() => {
		if (!selectedLanguage) return []
		return compatibleOutputKinds(selectedLanguage as ScriptLang)
	})

	let visibleOutputKinds = $derived(
		PIPELINE_OUTPUT_KINDS.filter((k) => compatibleKinds.includes(k.id))
	)

	// Popover closing doesn't unmount its content, so state persists across
	// opens. Reset everything back to initial state on close so the next
	// open starts fresh — otherwise reopening lands on the previous path
	// stage with the previous suffix, and a subsequent Enter creates a
	// duplicate of the first draft.
	function resetMenuState() {
		selectedKindId = kinds[0]?.id ?? ''
		stage = kinds[0]?.pickLanguage ? 'lang' : 'description'
		selectedLanguage = languages[defaultLangIdx()]?.lang
		selectedOutputKind = undefined
		pathSuffix = ''
		focusCol = 'lang'
		kindIdx = 0
		langIdx = defaultLangIdx()
		outputIdx = 0
	}

	function handleKindClick(k: PipelineInsertKind, close: () => void) {
		if (!k.pickLanguage) {
			onPick({ kindId: k.id })
			close()
			return
		}
		selectedKindId = k.id
		stage = 'lang'
		// Re-seed the language with the duckdb default rather than blanking
		// it — keeping the output column populated avoids a flash of empty
		// state when the user explores different trigger kinds.
		selectedLanguage = languages[defaultLangIdx()]?.lang
		selectedOutputKind = undefined
		pathSuffix = ''
		langIdx = defaultLangIdx()
		outputIdx = 0
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
		// Output kinds are language-dependent, so changing the language
		// invalidates any previously picked kind. We stay on the lang
		// stage and let the user click an output kind in the right
		// sub-column to advance.
		const idx = languages.findIndex((l) => l.lang === lang)
		if (idx >= 0) langIdx = idx
		if (pickOutputKind) {
			if (selectedLanguage !== lang) {
				selectedOutputKind = undefined
				outputIdx = 0
			}
			selectedLanguage = lang
			return
		}
		selectedLanguage = lang
		const base = defaultPathSuffix || 'pipeline_script'
		pathSuffix = `${base}_${shortSlug()}`
		stage = 'path'
		await focusPathInput()
	}

	async function handleOutputKindClick(kind: PipelineOutputKind) {
		const idx = visibleOutputKinds.findIndex((k) => k.id === kind)
		if (idx >= 0) outputIdx = idx
		selectedOutputKind = kind
		const base = defaultPathSuffix || 'pipeline_script'
		pathSuffix = `${base}_${shortSlug()}`
		stage = 'path'
		await focusPathInput()
	}

	function clampIdx(idx: number, len: number): number {
		if (len <= 0) return 0
		if (idx < 0) return 0
		if (idx >= len) return len - 1
		return idx
	}

	// Available column ids for keyboard nav, matching the actual rendered
	// columns: kind is hidden when there's only one option, output is only
	// rendered when pickOutputKind is true.
	function navColumns(): Array<'kind' | 'lang' | 'output'> {
		const cols: Array<'kind' | 'lang' | 'output'> = ['lang']
		if (kinds.length > 1) cols.unshift('kind')
		if (pickOutputKind) cols.push('output')
		return cols
	}

	function handleArrowKey(e: KeyboardEvent) {
		// Only the lang stage uses arrow nav — the path stage has its own
		// input-focused handler. Window-level keydown also fires while
		// typing in the path input, so we bail when the popover is closed
		// or focus is inside an editable element.
		if (!isOpen || stage !== 'lang') return
		const t = e.target as HTMLElement | null
		if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable)) return
		const cols = navColumns()
		// If somehow the focused column is no longer rendered (e.g. user
		// switched to a single-kind menu), snap back to lang before acting.
		if (!cols.includes(focusCol)) focusCol = 'lang'

		if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
			e.preventDefault()
			const dir = e.key === 'ArrowDown' ? 1 : -1
			if (focusCol === 'kind') {
				kindIdx = clampIdx(kindIdx + dir, kinds.length)
			} else if (focusCol === 'lang') {
				langIdx = clampIdx(langIdx + dir, languages.length)
				const lang = languages[langIdx]?.lang
				if (lang && pickOutputKind) {
					if (selectedLanguage !== lang) {
						selectedOutputKind = undefined
						outputIdx = 0
					}
					selectedLanguage = lang
				}
			} else if (focusCol === 'output') {
				outputIdx = clampIdx(outputIdx + dir, visibleOutputKinds.length)
			}
		} else if (e.key === 'ArrowRight' || e.key === 'ArrowLeft') {
			e.preventDefault()
			const i = cols.indexOf(focusCol)
			const next = e.key === 'ArrowRight' ? i + 1 : i - 1
			if (next < 0 || next >= cols.length) return
			const target = cols[next]
			// Output column needs a selected language to render anything;
			// preselect the focused lang if missing so right-arrow always
			// produces a useful destination.
			if (target === 'output' && !selectedLanguage) {
				const lang = languages[langIdx]?.lang
				if (!lang) return
				selectedLanguage = lang
				outputIdx = 0
			}
			focusCol = target
			if (target === 'output') outputIdx = clampIdx(outputIdx, visibleOutputKinds.length)
		} else if (e.key === 'Enter') {
			e.preventDefault()
			if (focusCol === 'kind') {
				const k = kinds[kindIdx]
				// Only the in-place transition (pickLanguage=true) is
				// reachable from keyboard — the close-and-emit path needs
				// the popover's `close` handle which isn't available here.
				if (k?.pickLanguage) {
					selectedKindId = k.id
					selectedLanguage = languages[defaultLangIdx()]?.lang
					selectedOutputKind = undefined
					langIdx = defaultLangIdx()
					outputIdx = 0
					focusCol = 'lang'
				}
			} else if (focusCol === 'lang') {
				const l = languages[langIdx]
				if (l) {
					if (pickOutputKind) {
						focusCol = 'output'
						outputIdx = clampIdx(outputIdx, visibleOutputKinds.length)
					} else {
						handleLanguageClick(l.lang)
					}
				}
			} else if (focusCol === 'output') {
				const k = visibleOutputKinds[outputIdx]
				if (k) handleOutputKindClick(k.id)
			}
		}
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
			stage = 'lang'
		}
	}
</script>

<svelte:window onkeydown={handleArrowKey} />

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
		isOpen = !!e.detail
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
					{#each kinds as k, i}
						{@const isSelected = selectedKindId === k.id}
						{@const isFocused = focusCol === 'kind' && kindIdx === i}
						<button
							type="button"
							onclick={() => {
								kindIdx = i
								focusCol = 'kind'
								handleKindClick(k, close)
							}}
							class={[
								'flex items-start gap-2 px-2 py-2 rounded-md text-left transition-colors',
								isSelected
									? 'bg-surface-selected text-emphasis'
									: 'hover:bg-surface-hover text-primary',
								isFocused ? 'ring-1 ring-emerald-400' : ''
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
					<!-- Two sub-columns when output-kind picking is enabled:
					     languages on the left, compatible output kinds on the
					     right. Selecting a language updates the output column
					     in place; selecting an output kind advances to path. -->
					<div
						class="flex flex-row grow min-w-0 overflow-hidden {pickOutputKind ? 'divide-x' : ''}"
					>
						<div
							class="flex flex-col gap-1 p-2 overflow-auto {pickOutputKind
								? 'w-44 shrink-0'
								: 'grow'}"
						>
							<div class="text-2xs font-normal text-secondary ml-2 mb-1">Language</div>
							{#each languages as l, i}
								{@const isSelected = pickOutputKind && selectedLanguage === l.lang}
								{@const isFocused = focusCol === 'lang' && langIdx === i}
								<Button
									variant="subtle"
									unifiedSize="sm"
									btnClasses="justify-start {isSelected ? 'bg-surface-selected' : ''} {isFocused
										? 'ring-1 ring-emerald-400'
										: ''}"
									onClick={() => {
										focusCol = 'lang'
										handleLanguageClick(l.lang)
									}}
								>
									<LanguageIcon lang={l.lang} width={14} height={14} />
									<span class="grow truncate text-left text-sm">{l.label}</span>
								</Button>
							{/each}
						</div>
						{#if pickOutputKind}
							<div class="flex flex-col gap-1 p-2 grow min-w-0 overflow-auto">
								<div class="text-2xs font-normal text-secondary ml-2 mb-1">Output asset</div>
								{#if !selectedLanguage}
									<span class="text-2xs text-tertiary px-2 py-1">
										Pick a language to see its output presets.
									</span>
								{:else if visibleOutputKinds.length === 0}
									<span class="text-2xs text-tertiary px-2">
										No output presets for this language.
									</span>
								{:else}
									{#each visibleOutputKinds as k, i}
										{@const isFocused = focusCol === 'output' && outputIdx === i}
										<button
											type="button"
											onclick={() => {
												focusCol = 'output'
												handleOutputKindClick(k.id)
											}}
											class={[
												'flex flex-col items-start gap-0.5 px-2 py-2 rounded-md text-left transition-colors hover:bg-surface-hover',
												isFocused ? 'ring-1 ring-emerald-400' : ''
											].join(' ')}
										>
											<span class="text-sm font-medium leading-tight">{k.label}</span>
											<span class="text-2xs text-tertiary font-normal leading-snug">
												{k.description}
											</span>
										</button>
									{/each}
								{/if}
							</div>
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
								title="Back"
								onClick={() => (stage = 'lang')}
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
