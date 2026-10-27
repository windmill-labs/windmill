<!--
@component
The composer's "Working in" picker: the folders, and the user's own space, this session's AI
changes items in. Several can be picked, and the menu stays open while they are; every one
picked is the whole workspace, folders created later included. Stays editable after the first
message; a change applies from the next turn.
-->
<script lang="ts">
	import { Check, ChevronDown, Folder } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import MenuItem from '$lib/components/meltComponents/MenuItem.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import type { MenubarMenuElements } from '@melt-ui/svelte'
	import { getAiChatManager } from './aiChatManagerContext'
	import { useWorkingInOptions } from './workingInOptions.svelte'
	import { scopeBadge, type ScopeOption } from './workingScope'

	type MeltItem = MenubarMenuElements['item']

	/** Past this many rows the menu gets a search field. */
	const SEARCH_FROM = 8

	let {
		ws,
		onlyWhenScoped = false
	}: {
		ws: string
		/** For the folded composer, which keeps a chip only while the session is scoped: a
		 * boundary must stay in sight, the default need not. */
		onlyWhenScoped?: boolean
	} = $props()

	const aiChatManager = getAiChatManager()
	const scope = useWorkingInOptions(aiChatManager, () => ws)

	let query = $state('')

	const current = $derived(aiChatManager.workingIn)
	const label = $derived(
		!current
			? 'All folders'
			: current.length === 0
				? 'No folders'
				: current.length === 1
					? current[0]
					: `${current[0]} +${current.length - 1}`
	)
	const searchable = $derived(scope.options.length > SEARCH_FROM)
	const shown = $derived(
		searchable && query.trim()
			? scope.options.filter((o) => o.root.toLowerCase().includes(query.trim().toLowerCase()))
			: scope.options
	)
	const own = $derived(shown.filter((o) => o.root.startsWith('u/')))
	const folders = $derived(shown.filter((o) => o.root.startsWith('f/')))

	function picked(root: string): boolean {
		return !current || current.includes(root)
	}

	/** Picking a root adds it to the scope or takes it out. With every listed root picked the
	 * session works in the whole workspace, so roots the menu does not list (a folder created
	 * later) stop asking too. */
	function toggle(root: string) {
		const all = scope.options.map((o) => o.root)
		const roots = current ?? all
		const next = roots.includes(root) ? roots.filter((r) => r !== root) : [...roots, root]
		aiChatManager.setWorkingIn(all.every((r) => next.includes(r)) ? undefined : next)
	}

	const ROW_CLASS =
		'w-full flex items-center gap-2 px-3 py-1.5 text-left text-xs text-primary font-normal hover:bg-surface-hover data-[highlighted]:bg-surface-hover rounded-sm transition-colors cursor-pointer'
	const ACTION_CLASS =
		'px-2 py-1 text-2xs text-secondary font-normal hover:bg-surface-hover data-[highlighted]:bg-surface-hover rounded-sm transition-colors cursor-pointer'
</script>

{#snippet sectionTitle(title: string)}
	<p class="px-3 pt-1.5 pb-1 text-2xs uppercase tracking-wide text-secondary">{title}</p>
{/snippet}

{#snippet option(item: MeltItem, o: ScopeOption)}
	{@const caption = [o.readOnly ? 'Read only' : '', scopeBadge(o)].filter(Boolean).join(' · ')}
	<MenuItem {item} class={ROW_CLASS} onClick={() => toggle(o.root)}>
		<div class="grow min-w-0">
			<p class="truncate">{o.root}</p>
			{#if caption}
				<p class="text-2xs text-secondary">{caption}</p>
			{/if}
		</div>
		{#if picked(o.root)}
			<Check size={14} class="shrink-0 text-primary" />
		{/if}
	</MenuItem>
{/snippet}

{#if aiChatManager.supportsWorkingIn && !aiChatManager.useModeActive && (current || !onlyWhenScoped)}
	<DropdownV2
		customMenu
		placement="top-start"
		fixedHeight={false}
		closeOnItemClick={false}
		on:close={() => (query = '')}
	>
		{#snippet buttonReplacement()}
			<Button
				nonCaptureEvent
				unifiedSize="2xs"
				variant="subtle"
				title="Working in: the AI creates and changes items only in these folders, and asks before changing anything elsewhere. It still reads the whole workspace."
				startIcon={{ icon: Folder }}
				endIcon={{ icon: ChevronDown }}
			>
				{label}
			</Button>
		{/snippet}
		{#snippet menu({ item })}
			<div
				class="bg-surface-tertiary dark:border w-64 rounded-lg shadow-lg focus:outline-none py-1 flex flex-col max-h-[60vh]"
			>
				<div class="px-3 pt-1.5 pb-1">
					<p class="text-xs font-semibold text-emphasis">Allow writes in</p>
					<p class="text-2xs text-secondary">Writes anywhere else ask you first.</p>
				</div>
				{#if searchable}
					<div class="px-3 pt-1 pb-1.5">
						<TextInput
							size="sm"
							bind:value={query}
							inputProps={{
								placeholder: 'Search folders',
								// Capture, not bubble: melt reads loose keys as typeahead before a delegated
								// handler would run. Escape still reaches the menu to close it.
								onkeydowncapture: (e) => {
									if (e.key !== 'Escape') e.stopPropagation()
								}
							}}
						/>
					</div>
				{/if}
				<div class="flex items-center justify-end gap-1 px-2">
					<MenuItem
						{item}
						class={ACTION_CLASS}
						onClick={() => aiChatManager.setWorkingIn(undefined)}
					>
						Select all
					</MenuItem>
					<MenuItem {item} class={ACTION_CLASS} onClick={() => aiChatManager.setWorkingIn([])}>
						Deselect all
					</MenuItem>
				</div>
				<div class="overflow-y-auto min-h-0">
					{#if own.length}
						{@render sectionTitle('Your space')}
						{#each own as o (o.root)}
							{@render option(item, o)}
						{/each}
					{/if}
					{#if folders.length}
						{#if own.length}
							<div class="my-1 border-t border-border-light"></div>
						{/if}
						{@render sectionTitle('Folders')}
						{#each folders as o (o.root)}
							{@render option(item, o)}
						{/each}
					{/if}
					{#if !shown.length}
						<p class="px-3 py-1.5 text-2xs text-secondary">No folder matches</p>
					{/if}
				</div>
			</div>
		{/snippet}
	</DropdownV2>
{/if}
