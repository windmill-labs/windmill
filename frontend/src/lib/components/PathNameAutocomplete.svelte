<script module lang="ts">
	import type { ButtonType } from '$lib/components/common/button/model'
	import { PathAutocompleteService } from '$lib/gen'

	type InputSize = ButtonType.UnifiedSize

	export type AutocompleteSegment = { name: string; is_folder: boolean }

	/** Client-side TTL for the workspace path list (mirrors backend cache). */
	const PATH_LIST_TTL_MS = 60_000

	type Entry = {
		at: number
		paths: string[] | null
		pending: Promise<string[]> | null
	}

	/** Module-level shared cache so multiple PathNameAutocomplete instances on
	 * the same page reuse a single fetch per workspace. */
	const pathListCache = new Map<string, Entry>()

	export async function fetchWorkspacePaths(workspace: string): Promise<string[]> {
		const now = Date.now()
		const existing = pathListCache.get(workspace)
		if (existing) {
			if (existing.paths && now - existing.at < PATH_LIST_TTL_MS) return existing.paths
			if (existing.pending) return existing.pending
		}
		const pending = (async () => {
			try {
				const res = await PathAutocompleteService.listPathAutocompletePaths({ workspace })
				const paths = res.paths ?? []
				pathListCache.set(workspace, { at: Date.now(), paths, pending: null })
				return paths
			} catch (_e) {
				pathListCache.delete(workspace)
				return []
			}
		})()
		pathListCache.set(workspace, { at: now, paths: null, pending })
		return pending
	}

	export function invalidateWorkspacePaths(workspace: string) {
		pathListCache.delete(workspace)
	}

	/** Derive the set of path segments that exist directly under a given folder
	 * prefix, from the flat list of all workspace paths. */
	export function segmentsUnderPrefix(
		paths: string[],
		folderPrefix: string
	): AutocompleteSegment[] {
		const seen = new Map<string, boolean>()
		const prefixLen = folderPrefix.length
		for (const p of paths) {
			if (!p.startsWith(folderPrefix)) continue
			const remainder = p.slice(prefixLen)
			if (remainder.length === 0) continue
			const slash = remainder.indexOf('/')
			if (slash >= 0) {
				seen.set(remainder.slice(0, slash), true)
			} else if (!seen.has(remainder)) {
				seen.set(remainder, false)
			}
		}
		return Array.from(seen, ([name, is_folder]) => ({ name, is_folder })).sort((a, b) =>
			a.name.localeCompare(b.name)
		)
	}
</script>

<script lang="ts">
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { workspaceStore } from '$lib/stores'
	import { untrack } from 'svelte'

	type Props = {
		prefix: string
		value?: string
		placeholder?: string
		disabled?: boolean
		autofocus?: boolean
		id?: string
		size?: InputSize
		error?: string | boolean
		onkeyup?: (e: KeyboardEvent) => void
	}

	let {
		prefix,
		value = $bindable(''),
		placeholder = '',
		disabled = false,
		autofocus = false,
		id,
		size = 'md',
		error,
		onkeyup
	}: Props = $props()

	let inputEl: TextInput | undefined = $state(undefined)
	export function focus() {
		inputEl?.focus()
	}

	let allPaths = $state<string[]>([])
	let cycleMode = $state<{
		cyclePrefix: string
		options: AutocompleteSegment[]
		index: number
	} | null>(null)
	let dismissedFor: string | undefined = $state(undefined)
	let hasFocus = $state(false)
	// Flag set when keydown consumes Enter for ghost-text acceptance.
	// Checked in keyup to suppress Path.svelte's 'enter' dispatch.
	let enterConsumed = false

	let currentSegment = $derived.by(() => {
		const v = value ?? ''
		const idx = v.lastIndexOf('/')
		return idx >= 0 ? v.slice(idx + 1) : v
	})

	let folderPrefix = $derived.by(() => {
		const v = value ?? ''
		const idx = v.lastIndexOf('/')
		return prefix + (idx >= 0 ? v.slice(0, idx + 1) : '')
	})

	let fullPrefix = $derived(prefix + (value ?? ''))

	let derivedFolderMatches = $derived.by(() => {
		if (allPaths.length === 0) return []
		return segmentsUnderPrefix(allPaths, folderPrefix).filter(
			(s) => s.is_folder && s.name.startsWith(currentSegment)
		)
	})

	let displayedOptions = $derived(cycleMode ? cycleMode.options : derivedFolderMatches)
	let displayedActiveIndex = $derived(cycleMode ? cycleMode.index : -1)

	// Ghost text: longest common prefix across all matching folder names,
	// beyond what the user has already typed. Shows the unambiguous part
	// the user can accept with Enter (multi-match) or Tab (single match).
	let ghostText = $derived.by(() => {
		if (cycleMode) return ''
		if (derivedFolderMatches.length === 0) return ''
		const names = derivedFolderMatches.map((s) => s.name)
		// Compute LCP of all matching folder names
		let lcp = names[0]
		for (let i = 1; i < names.length; i++) {
			let j = 0
			while (j < lcp.length && j < names[i].length && lcp[j] === names[i][j]) j++
			lcp = lcp.slice(0, j)
		}
		if (lcp.length <= currentSegment.length) return ''
		const completion = lcp.slice(currentSegment.length)
		// Append '/' only when the LCP fully resolves to one folder name
		const trailingSlash = derivedFolderMatches.length === 1 ? '/' : ''
		return completion + trailingSlash
	})

	let showList = $derived(
		!disabled && dismissedFor !== fullPrefix && hasFocus && displayedOptions.length > 0
	)

	function applyCycleOrComplete(direction: 1 | -1): boolean {
		if (cycleMode) {
			const n = cycleMode.options.length
			const nextIdx = (cycleMode.index + direction + n) % n
			cycleMode = { ...cycleMode, index: nextIdx }
			value = cycleMode.cyclePrefix + cycleMode.options[nextIdx].name + '/'
			return true
		}
		if (derivedFolderMatches.length === 0) return false
		const opts = [...derivedFolderMatches]
		const vNow = value ?? ''
		const slash = vNow.lastIndexOf('/')
		const cyclePrefix = slash >= 0 ? vNow.slice(0, slash + 1) : ''
		if (opts.length === 1) {
			value = cyclePrefix + opts[0].name + '/'

			return true
		}
		const idx = direction === 1 ? 0 : opts.length - 1
		cycleMode = { cyclePrefix, options: opts, index: idx }
		value = cyclePrefix + opts[idx].name + '/'

		return true
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Tab' && !e.ctrlKey && !e.metaKey && !e.altKey) {
			const dir: 1 | -1 = e.shiftKey ? -1 : 1
			if (applyCycleOrComplete(dir)) {
				e.preventDefault()
				return
			}
			return
		}
		// Enter selects the first (or currently highlighted) folder and
		// navigates into it, showing its children immediately.
		if (e.key === 'Enter' && !e.ctrlKey && !e.metaKey && derivedFolderMatches.length > 0) {
			e.preventDefault()
			e.stopPropagation()
			enterConsumed = true
			const vNow = value ?? ''
			const slash = vNow.lastIndexOf('/')
			const committed = slash >= 0 ? vNow.slice(0, slash + 1) : ''
			const pick = derivedFolderMatches[0]
			value = committed + pick.name + '/'
			cycleMode = null
			return
		}
		if (e.key === 'Escape' && (cycleMode || showList || ghostText)) {
			e.stopPropagation()
			cycleMode = null
			dismissedFor = fullPrefix
			return
		}
	}

	function selectOption(opt: AutocompleteSegment) {
		const vNow = value ?? ''
		const slash = vNow.lastIndexOf('/')
		const valueCommitted = slash >= 0 ? vNow.slice(0, slash + 1) : ''
		value = valueCommitted + opt.name + '/'
		cycleMode = null

		inputEl?.focus()
	}

	async function loadPaths(workspace: string) {
		const paths = await fetchWorkspacePaths(workspace)
		// Guard against workspace changing during the in-flight fetch.
		if ($workspaceStore === workspace) allPaths = paths
	}

	$effect(() => {
		const ws = $workspaceStore
		if (ws) void loadPaths(ws)
	})

	$effect(() => {
		void value
		void prefix
		// Re-arm after the dismissed prefix is left.
		if (untrack(() => dismissedFor) !== undefined) {
			const curr = untrack(() => fullPrefix)
			if (untrack(() => dismissedFor) !== curr) dismissedFor = undefined
		}
		// Exit cycle mode if the value no longer matches the cycled option.
		const cm = untrack(() => cycleMode)
		if (cm) {
			const expected = cm.cyclePrefix + cm.options[cm.index].name + '/'
			if ((value ?? '') !== expected) cycleMode = null
		}
	})

	const overlayPaddingX: Record<InputSize, string> = {
		xs: 'px-1',
		sm: 'px-2',
		md: 'px-2',
		lg: 'px-2'
	}
	const overlayHeight: Record<InputSize, string> = {
		xs: 'h-5',
		sm: 'h-7',
		md: 'h-8',
		lg: 'h-10'
	}

	function onInputFocus() {
		hasFocus = true
		// Opportunistic refresh if the cache is stale.
		if ($workspaceStore) void loadPaths($workspaceStore)
	}
	function onInputBlur() {
		setTimeout(() => {
			hasFocus = false
		}, 150)
	}
</script>

<div class="relative w-full">
	<div class="relative w-full">
		<TextInput
			bind:this={inputEl}
			bind:value
			{size}
			{error}
			inputProps={{
				disabled,
				type: 'text',
				id,
				autofocus,
				autocomplete: 'off',
				spellcheck: false,
				placeholder: showList || ghostText ? '' : placeholder,
				onkeydown: handleKeydown,
				onkeyup: (e: KeyboardEvent) => {
					if (e.key === 'Enter' && enterConsumed) {
						e.preventDefault()
						e.stopPropagation()
						enterConsumed = false
						return
					}
					onkeyup?.(e)
				},
				onfocus: onInputFocus,
				onblur: onInputBlur
			}}
		/>
		{#if ghostText && hasFocus && !cycleMode}
			<div
				class="pointer-events-none absolute inset-0 flex items-center overflow-hidden whitespace-pre font-normal text-xs text-primary {overlayPaddingX[
					size
				]} {overlayHeight[size]}"
				aria-hidden="true"
			>
				<span class="invisible">{value ?? ''}</span>
				<span class="text-tertiary/70">{ghostText}</span>
				<span
					class="ml-1.5 px-1 py-0 rounded border border-border-light text-[10px] text-tertiary bg-surface-secondary"
				>
					Enter
				</span>
			</div>
		{/if}
	</div>
	{#if showList}
		<div
			class="absolute left-0 top-full z-10 mt-1 flex max-w-full flex-wrap items-center gap-1 rounded border border-border-light bg-surface px-1.5 py-1 shadow-sm"
		>
			<span class="text-[10px] text-tertiary mr-0.5">
				{cycleMode ? 'Tab to cycle' : 'Tab'}
			</span>
			{#each displayedOptions as opt, i (opt.name)}
				<button
					type="button"
					tabindex="-1"
					onmousedown={(e) => e.preventDefault()}
					onclick={() => selectOption(opt)}
					class="px-1.5 py-0 rounded border text-[11px] font-mono leading-5 transition-colors
						{i === displayedActiveIndex
						? 'border-border-selected bg-surface-selected text-primary'
						: 'border-border-light bg-surface-secondary text-secondary hover:border-border-selected'}"
				>
					{opt.name}/
				</button>
			{/each}
		</div>
	{/if}
</div>
