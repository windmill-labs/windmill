<script lang="ts">
	import type { Snippet } from 'svelte'
	import {
		ChevronDown,
		ChevronUp,
		ChevronsUpDown,
		Folder,
		Plus,
		Search,
		User,
		Users
	} from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { ON_HOVER, type Config } from './model'
	import type { Owner, TreeItem } from './fixtures'
	import KindIcon from './KindIcon.svelte'
	import StarToggle from './StarToggle.svelte'

	interface Props {
		owners: Owner[]
		config: Config
		/** Paths starred in the playground, so a row's star agrees with the cards'. */
		starred: Set<string>
		onToggleStar: (item: TreeItem) => void
		/** A band rendered between the controls and the tree. */
		band?: Snippet
		/** Only the first owners, for the stacked comparison. */
		preview?: boolean
	}

	let { owners, config, starred, onToggleStar, band, preview = false }: Props = $props()

	let open = $state<Record<string, boolean>>({})
	let flat = $derived(config.listFrame === 'flat')
	let pad = $derived(flat ? '' : 'px-5')
	let shownOwners = $derived(preview ? owners.slice(0, 2) : owners)

	// Owner chips as ListFilters shows them: your space, folders, then each other user.
	let chips = $derived([
		...owners
			.filter((o) => o.icon !== 'users')
			.map((o) => ({ name: o.name, user: o.icon === 'user' })),
		...[
			...new Set(
				owners
					.filter((o) => o.icon === 'users')
					.flatMap((o) => o.items.map((i) => i.path.split('/').slice(0, 2).join('/')))
			)
		].map((name) => ({ name, user: true }))
	])

	const ROW_MENU = [
		{ displayName: 'Edit', action: () => {} },
		{ displayName: 'View runs', action: () => {} },
		{ displayName: 'Move/Rename', action: () => {} },
		{ displayName: 'Archive', action: () => {} }
	]
</script>

{#snippet treeRow(item: TreeItem)}
	<div
		class="group inline-flex w-full items-center gap-4 rounded-md px-4 py-3 hover:bg-surface-hover"
		style="padding-left: 32px;"
	>
		<KindIcon kind={item.kind} size={16} muted={config.icons === 'muted'} />
		<a href="#{item.path}" class="min-w-0 grow">
			<div class="flex items-center gap-1.5 text-left text-xs font-semibold text-emphasis">
				<span class="truncate">{item.summary || item.path}</span>
				<StarToggle
					starred={starred.has(item.path)}
					onToggle={() => onToggleStar(item)}
					class="inline-flex shrink-0 p-px text-yellow-500 opacity-0 data-[starred]:opacity-100 group-hover:opacity-100"
				/>
			</div>
			<div class="truncate text-left text-3xs font-normal text-hint">{item.path}</div>
		</a>
		{#if item.badges.length}
			<div class="hidden shrink-0 flex-row items-center gap-4 lg:flex">
				{#each item.badges as badge (badge)}
					<span
						class="rounded-md px-2 py-0.5 text-2xs {badge === 'Draft'
							? 'bg-blue-50 text-blue-800 dark:bg-blue-700/40 dark:text-blue-100'
							: 'bg-surface-secondary text-primary'}"
					>
						{badge}
					</span>
				{/each}
			</div>
		{/if}
		<div
			class="flex shrink-0 items-center text-secondary {config.menu === 'hover' ? ON_HOVER : ''}"
		>
			<DropdownV2 items={ROW_MENU} size="xs" fixedHeight={false} />
		</div>
	</div>
{/snippet}

<div
	class="flex flex-col {flat ? '' : 'rounded-lg border border-border-light bg-surface-tertiary'}"
>
	<div class="flex flex-wrap items-center gap-2 py-3 {pad}">
		<div class="relative w-full basis-full sm:min-w-[200px] sm:grow sm:basis-0">
			<div
				class="flex h-8 items-center rounded-md border border-border-light bg-surface-input px-2.5 text-xs text-hint"
			>
				<span class="flex-1 truncate">Search Scripts, Flows & Apps</span>
				<Search size={16} class="text-secondary" />
			</div>
		</div>
		<Button unifiedSize="md" variant="default" endIcon={{ icon: ChevronDown }}>All kinds</Button>
		<Button
			unifiedSize="md"
			variant="default"
			iconOnly
			title="Expand all"
			startIcon={{ icon: ChevronsUpDown }}
		/>
		<Button
			unifiedSize="md"
			variant="accent"
			startIcon={{ icon: Plus }}
			endIcon={{ icon: ChevronDown }}
		>
			New
		</Button>
	</div>
	{#if band}
		<div class="pb-3 {pad}">{@render band()}</div>
	{/if}
	{#if config.chips && chips.length}
		<div class="flex w-full flex-wrap gap-x-2 gap-y-0.5 pb-3 {pad}">
			{#each chips as chip (chip.name)}
				<span
					class="inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-2xs text-primary hover:bg-surface-hover"
				>
					{#if chip.user}<User size={12} />{:else}<Folder size={12} />{/if}
					{chip.name}
				</span>
			{/each}
		</div>
	{/if}
	{#if (config.chips && chips.length) || band}
		<div class="border-b {flat ? '' : 'mx-5'}"></div>
	{/if}
	<div class={flat ? '-mx-4' : 'px-1'}>
		{#if owners.length === 0}
			<div class="px-4 py-4">
				<div class="rounded-md border-[1.5px] border-dashed border-border-normal/60 bg-surface">
					{#each [1, 0.6, 0.3] as opacity, i (i)}
						<div
							class="flex items-center gap-[14px] px-4 py-[13px] {i > 0
								? 'border-t border-dashed border-border-light'
								: ''}"
							style="opacity: {opacity}"
						>
							<div class="size-4 shrink-0 rounded bg-border-light"></div>
							<div>
								<div class="h-[9px] w-[140px] rounded-full bg-border-light"></div>
								<div class="mt-[5px] h-[7px] w-[70px] rounded-full bg-border-light/60"></div>
							</div>
						</div>
					{/each}
					<div
						class="border-t border-dashed border-border-light px-4 pb-[22px] pt-[18px] text-center text-[13.5px] leading-relaxed text-hint"
					>
						Your scripts, flows and apps will show up here.
						<span class="text-accent">Start from a template</span> or
						<span class="text-accent">create a new one</span>.
					</div>
				</div>
			</div>
		{:else}
			<div class="py-1">
				{#each shownOwners as owner (owner.name)}
					{@const OwnerIcon =
						owner.icon === 'folder' ? Folder : owner.icon === 'users' ? Users : User}
					<!-- svelte-ignore a11y_click_events_have_key_events -->
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<div
						onclick={() => (open[owner.name] = !open[owner.name])}
						class="flex w-full cursor-pointer flex-row items-center justify-between rounded-md px-4 py-2 hover:bg-surface-hover"
					>
						<div class="flex flex-row items-center gap-4">
							<OwnerIcon size={16} class="shrink-0 text-secondary" />
							{#if config.treeRows === 'one'}
								<div class="flex h-5 items-baseline gap-2">
									<span class="whitespace-nowrap text-xs font-semibold text-emphasis">
										{owner.name}
									</span>
									<span class="whitespace-nowrap text-2xs font-normal text-hint">
										{owner.detail}
									</span>
								</div>
							{:else}
								<div>
									<span class="whitespace-nowrap text-xs font-semibold text-emphasis">
										{owner.name}
									</span>
									<div class="whitespace-nowrap text-2xs font-normal text-secondary">
										({owner.detail})
									</div>
								</div>
							{/if}
						</div>
						{#if open[owner.name]}
							<ChevronUp size={16} />
						{:else}
							<ChevronDown size={16} />
						{/if}
					</div>
					{#if open[owner.name]}
						{#each owner.items as item (item.path)}
							{@render treeRow(item)}
						{/each}
					{/if}
				{/each}
			</div>
		{/if}
	</div>
</div>
