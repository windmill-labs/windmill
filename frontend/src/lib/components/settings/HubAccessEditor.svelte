<script lang="ts">
	import { Alert, Button, Skeleton } from '$lib/components/common'
	import Checkbox from '$lib/components/common/checkbox/Checkbox.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { createAsyncConfirmationModal } from '$lib/components/common/confirmationModal/asyncConfirmationModal.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import IconedResourceType from '$lib/components/IconedResourceType.svelte'
	import { listHubIntegrationsShared } from '$lib/components/displayNameLoaders'
	import { integrationDisplayName } from '$lib/components/resourceTypeDisplay'
	import { disableHubStore, hubBaseUrlStore } from '$lib/stores'
	import type { HubAccess, HubAccessList, ListHubIntegrationsResponse } from '$lib/gen'
	import { ChevronDown, ExternalLink, SquareCheckBig, X } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { fly } from 'svelte/transition'

	type Level = HubAccess | 'none'

	interface Props {
		value: HubAccessList
		/** The list as saved, which a row that differs from it shows struck through. */
		saved: HubAccessList
		/** Only lists what is allowed, with nothing to change. */
		readOnly?: boolean
		onchange: (value: HubAccessList) => void
	}

	let { value, saved, readOnly = false, onchange }: Props = $props()

	const BULK_WRITE_CONFIRM = 10
	const COUNT_CAP = 500

	type HubEntry = ListHubIntegrationsResponse[number]
	/** Hub entries by lowercased name, which is how the server reads the list and the Hub's
	 *  integration page its slug. The Hub can hold both `Slack` and `slack`, and counts each apart. */
	let hubNames: Record<string, HubEntry[]> = $state({})
	let loading = $state(!$disableHubStore)
	let hubUnavailable = $state($disableHubStore)
	if (!$disableHubStore) {
		listHubIntegrationsShared()
			.then(
				(i) => {
					const names: Record<string, HubEntry[]> = {}
					for (const e of i) (names[e.name.toLowerCase()] ??= []).push(e)
					hubNames = names
				},
				() => (hubUnavailable = true)
			)
			.finally(() => (loading = false))
	}

	// A listed integration shows even if the Hub is down or no longer has it. The saved ones come
	// first, and a row does not move while its level changes.
	const all = $derived.by(() => {
		const first = new Set(Object.keys(saved))
		const names = readOnly
			? Object.keys(value)
			: [...first, ...Object.keys(value), ...Object.keys(hubNames)]
		return Array.from(new Set(names))
			.map((name) => ({ name, label: integrationDisplayName(name) }))
			.sort(
				(a, b) =>
					Number(first.has(b.name)) - Number(first.has(a.name)) || a.label.localeCompare(b.label)
			)
	})

	const level = (name: string): Level => value[name] ?? 'none'
	const savedLevel = (name: string): Level => saved[name] ?? 'none'
	const LEVEL_LABELS: Record<Level, string> = { none: 'None', read: 'Read', write: 'Write' }
	const LEVEL_TOOLTIPS: Record<Level, string> = {
		none: 'No script',
		read: 'Only the scripts that do not change data',
		write: 'Every approved script'
	}

	const VIEWS: { id: string; label: string; keep: (name: string) => boolean; empty: string }[] = [
		{ id: 'all', label: 'All integrations', keep: () => true, empty: 'No integrations here.' },
		{
			id: 'allowed',
			label: 'Allowed',
			keep: (n) => level(n) !== 'none',
			empty: 'No integration is allowed yet.'
		},
		{
			id: 'none',
			label: 'Not allowed',
			keep: (n) => level(n) === 'none',
			empty: 'Every integration is allowed.'
		},
		{
			id: 'changed',
			label: 'Changed',
			keep: (n) => level(n) !== savedLevel(n),
			empty: 'Nothing changed yet.'
		}
	]
	let viewId = $state('all')
	// Read-only hides the view menu, so the one left selected must not filter the list.
	const view = $derived((!readOnly && VIEWS.find((v) => v.id === viewId)) || VIEWS[0])
	let search = $state('')
	const query = $derived(search.trim().toLowerCase())
	const found = $derived(
		all.filter(
			({ name, label }) =>
				query === '' || name.includes(query) || label.toLowerCase().includes(query)
		)
	)
	const shown = $derived(found.filter(({ name }) => view.keep(name)))

	// Only what is on screen can be selected, so a bulk change never reaches a hidden row.
	let selected: string[] = $state([])
	const selection = $derived(shown.filter((i) => selected.includes(i.name)).map((i) => i.name))
	// The level every selected row has, if they all have the same.
	const selectionLevel = $derived.by(() => {
		const levels = new Set(selection.map(level))
		return levels.size === 1 ? [...levels][0] : undefined
	})
	// Remounts the bulk control after a refused change, which it would otherwise show as made.
	let bulkControlKey = $state(0)
	let lastToggled: string | undefined = undefined
	let shiftHeld = false

	function toggleSelected(name: string) {
		const names = shown.map((i) => i.name)
		const from = lastToggled === undefined ? -1 : names.indexOf(lastToggled)
		if (shiftHeld && from >= 0) {
			const to = names.indexOf(name)
			const range = names.slice(Math.min(from, to), Math.max(from, to) + 1)
			selected = Array.from(new Set([...selection, ...range]))
		} else {
			selected = selection.includes(name)
				? selection.filter((n) => n !== name)
				: [...selection, name]
		}
		lastToggled = name
	}

	function apply(names: string[], to: Level) {
		const next = { ...value }
		for (const n of names) {
			if (to === 'none') delete next[n]
			else next[n] = to
		}
		onchange(next)
	}

	const plural = (n: number) => `${n} integration${n === 1 ? '' : 's'}`

	const confirmation = createAsyncConfirmationModal()

	async function setMany(names: string[], to: Level) {
		const changed = names.filter((n) => level(n) !== to)
		if (changed.length === 0) return
		const raised = changed.filter((n) => savedLevel(n) !== 'write')
		if (to === 'write' && raised.length > BULK_WRITE_CONFIRM) {
			const ok = await confirmation.ask({
				title: `Allow writing on ${plural(raised.length)}?`,
				confirmationText: 'Set to write',
				type: 'info',
				children:
					'<span>Members whose role cannot write code may then use all of their approved Hub scripts, including those that change data.</span>'
			})
			if (!ok) {
				bulkControlKey++
				return
			}
		}
		apply(changed, to)
	}

	const changes = $derived.by(() => {
		const names = Array.from(new Set([...Object.keys(saved), ...Object.keys(value)]))
		const count = (f: (n: string) => boolean) => names.filter(f).length
		return [
			[count((n) => !saved[n]), 'added'],
			[count((n) => !!saved[n] && !!value[n] && saved[n] !== value[n]), 'changed'],
			[count((n) => !value[n]), 'removed']
		]
			.filter(([n]) => n)
			.map(([n, what]) => `${n} ${what}`)
			.join(', ')
	})

	const summary = $derived.by(() => {
		const levels = Object.values(value)
		const read = levels.filter((l) => l === 'read').length
		return levels.length === 0
			? 'No Hub scripts are allowed'
			: `${levels.length} allowed: ${read} read, ${levels.length - read} write`
	})
	const everyAllowed = $derived(
		!readOnly && !hubUnavailable && all.every(({ name }) => level(name) !== 'none')
	)

	const capped = (n: number) => (n > COUNT_CAP ? `${COUNT_CAP}+` : `${n}`)
	/** Summed over every spelling; nothing on a Hub that does not count. */
	function countsLabel(name: string): string | undefined {
		const entries = hubNames[name] ?? []
		if (entries.length === 0 || entries.some((e) => e.approved_scripts === undefined)) return
		const total = entries.reduce((n, e) => n + (e.approved_scripts ?? 0), 0)
		const readOnly = entries.reduce((n, e) => n + (e.read_only_scripts ?? 0), 0)
		return total === 0
			? 'No approved script'
			: `${capped(total)} ${total === 1 ? 'script' : 'scripts'}, ${capped(readOnly)} read-only`
	}
</script>

{#snippet icon(name: string)}
	<!-- The fallback icon ignores its size. -->
	<div class="shrink-0 [&_svg]:size-[18px]">
		<IconedResourceType {name} silent height="18px" width="18px" />
	</div>
{/snippet}

<div class="flex flex-col gap-2">
	<div class="flex flex-wrap items-center gap-2">
		<TextInput
			bind:value={search}
			size="sm"
			class="w-52 max-w-full"
			inputProps={{ placeholder: 'Search integrations', 'aria-label': 'Search integrations' }}
		/>
		{#if !readOnly}
			<DropdownV2
				placement="bottom-start"
				fixedHeight={false}
				items={VIEWS.map((v) => ({
					displayName: v.label,
					selected: v.id === view.id,
					action: () => (viewId = v.id)
				}))}
			>
				{#snippet buttonReplacement()}
					<Button
						nonCaptureEvent
						variant="default"
						unifiedSize="sm"
						title="Filter the list"
						endIcon={{ icon: ChevronDown }}
					>
						{view.label}
					</Button>
				{/snippet}
			</DropdownV2>
		{/if}
	</div>

	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class={twMerge(
			'flex rounded-md border border-border-light overflow-hidden',
			readOnly ? 'max-h-[320px]' : 'h-[320px]'
		)}
		onkeydown={(e) => {
			if (e.key === 'Escape' && selection.length > 0) {
				e.preventDefault()
				selected = []
			}
		}}
	>
		<div class="relative flex flex-col grow min-w-0 min-h-0 bg-surface-tertiary">
			<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
			<div
				class={twMerge('grow min-h-0 overflow-y-auto', selection.length > 0 ? 'pb-14' : '')}
				tabindex={readOnly ? undefined : 0}
				aria-label="Integrations"
				onkeydown={(e) => {
					if (!readOnly && (e.ctrlKey || e.metaKey) && e.key === 'a') {
						e.preventDefault()
						selected = shown.map((i) => i.name)
					}
				}}
			>
				{#if hubUnavailable && !readOnly}
					<div class="p-3">
						<Alert type="info" size="xs" title="The Hub is unavailable">
							Only the integrations already allowed are shown.
						</Alert>
					</div>
				{/if}
				{#if loading && !readOnly}
					<div class="p-3"><Skeleton layout={[[2], 0.5, [2], 0.5, [2]]} /></div>
				{:else}
					<div class="flex flex-col divide-y divide-border-light">
						{#each shown as { name, label } (name)}
							{@const page = hubNames[name] && `${$hubBaseUrlStore}/integrations/${name}`}
							{@const count = countsLabel(name)}
							<div class="group/row flex items-center gap-3 px-3 py-1.5 hover:bg-surface-hover">
								<!-- As on Home: the icon until the row is hovered or anything is selected, then
								     the checkbox, stacked so nothing shifts. -->
								<div class="shrink-0 relative size-[18px]">
									<div
										class={twMerge(
											'absolute inset-0',
											readOnly
												? ''
												: selection.length > 0
													? 'invisible'
													: 'group-hover/row:invisible'
										)}
									>
										{@render icon(name)}
									</div>
									{#if !readOnly}
										<Checkbox
											class={twMerge(
												'absolute inset-px w-4 h-4',
												selection.length > 0 ? '' : 'invisible group-hover/row:visible'
											)}
											checked={selection.includes(name)}
											title={selection.includes(name)
												? 'Deselect'
												: 'Select (shift-click to select a range)'}
											onClick={(e) => (shiftHeld = e.shiftKey)}
											onChange={() => toggleSelected(name)}
										/>
									{/if}
								</div>
								<div class="grow min-w-0 flex flex-wrap items-center gap-x-2">
									{#if page}
										<Button
											variant="subtle"
											unifiedSize="xs"
											href={page}
											target="_blank"
											title="Open its scripts on the Hub"
											endIcon={{ icon: ExternalLink, classes: 'text-hint' }}
											wrapperClasses="min-w-0"
											btnClasses="min-w-0 px-0 hover:bg-transparent hover:underline"
										>
											<span class="text-xs font-semibold text-emphasis truncate">{label}</span>
										</Button>
									{:else}
										<span class="text-xs font-semibold text-emphasis truncate">{label}</span>
									{/if}
									{#if count}
										<span class="text-2xs text-secondary">{count}</span>
									{/if}
								</div>
								{#if level(name) !== savedLevel(name)}
									<span class="shrink-0 text-2xs text-secondary" title="Saved value">
										<span class="line-through">{LEVEL_LABELS[savedLevel(name)]}</span> →
									</span>
								{/if}
								<div class="shrink-0">
									<!-- Bound, not passed: once clicked, the group keeps its own copy of a plain
									     prop and misses a bulk change. The setter also hears bulk changes. -->
									<ToggleButtonGroup
										noWFull
										disabled={readOnly}
										bind:selected={
											() => level(name), (v) => v !== level(name) && apply([name], v as Level)
										}
									>
										{#snippet children({ item })}
											<ToggleButton value="none" label="None" small {item} />
											<ToggleButton value="read" label="Read" small {item} />
											<ToggleButton value="write" label="Write" small {item} />
										{/snippet}
									</ToggleButtonGroup>
								</div>
							</div>
						{:else}
							<p class="px-3 py-4 text-xs text-secondary">
								{query !== '' ? `No integration matches "${search.trim()}".` : view.empty}
							</p>
						{/each}
					</div>
				{/if}
			</div>

			{#if selection.length > 0}
				<div class="absolute bottom-3 inset-x-0 flex justify-center pointer-events-none">
					<div
						transition:fly={{ y: 16, duration: 150 }}
						class="pointer-events-auto flex items-center gap-2 rounded-md border bg-surface px-2 py-1.5 shadow-lg whitespace-nowrap"
					>
						<span class="text-xs font-semibold text-emphasis">
							{selection.length < shown.length
								? `${selection.length} selected`
								: `All ${shown.length} selected`}
						</span>
						<div class="h-4 border-l"></div>
						<span class="text-xs text-secondary">Set to</span>
						{#key bulkControlKey}
							<!-- The group ignores `undefined`, so a mixed selection is a value no button has. -->
							<ToggleButtonGroup noWFull selected={selectionLevel ?? ''}>
								{#snippet children({ item })}
									{#each ['none', 'read', 'write'] as const as to (to)}
										<ToggleButton
											value={to}
											label={LEVEL_LABELS[to]}
											tooltip={LEVEL_TOOLTIPS[to]}
											small
											{item}
											onActivate={() => setMany(selection, to)}
										/>
									{/each}
								{/snippet}
							</ToggleButtonGroup>
						{/key}
						<div class="h-4 border-l"></div>
						{#if selection.length < shown.length}
							<Button
								variant="subtle"
								unifiedSize="sm"
								startIcon={{ icon: SquareCheckBig }}
								title="Select every integration shown"
								onclick={() => (selected = shown.map((i) => i.name))}
							>
								All
							</Button>
						{/if}
						<Button
							variant="subtle"
							unifiedSize="sm"
							iconOnly
							startIcon={{ icon: X }}
							title="Clear selection (Esc)"
							aria-label="Clear selection"
							onclick={() => (selected = [])}
						/>
					</div>
				</div>
			{/if}
		</div>
	</div>

	<span class="text-xs text-secondary">
		{summary}{#if changes !== ''}. <span class="font-semibold text-emphasis">{changes}</span
			>{#if everyAllowed}. New Hub integrations start at None{/if}{/if}
	</span>
</div>

<ConfirmationModal {...confirmation.props} />
