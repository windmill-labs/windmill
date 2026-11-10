<!--
@component
The starred items as a sidebar section, turned on from Customize in the sidebar menu.
-->
<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import { Bot, CodeXml, LayoutDashboard, Star, Table2, Trash2 } from 'lucide-svelte'
	import { page } from '$app/state'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { globalDbManagerDrawer, workspaceStore } from '$lib/stores'
	import { parseDbInputFromAssetSyntax } from '$lib/utils'
	import { useLocalStorageValue } from '$lib/svelte5Utils.svelte'
	import SidebarSection from './SidebarSection.svelte'
	import { favoriteManager } from './FavoriteMenu.svelte'
	import { sidebarClasses } from './MenuButton.svelte'
	import { sidebarPrefs } from './sidebarPrefs.svelte'

	let { isCollapsed = false }: { isCollapsed?: boolean } = $props()

	const sectionCollapsed = useLocalStorageValue(
		'windmill_favorites_section_collapsed',
		false,
		'boolean'
	)

	const loaded = $derived(favoriteManager.loadedFor === $workspaceStore)
</script>

{#if sidebarPrefs.favorites.val && loaded}
	<SidebarSection
		title="Favorites"
		icon={Star}
		folded={sectionCollapsed.val}
		onToggle={() => (sectionCollapsed.val = !sectionCollapsed.val)}
		{isCollapsed}
	>
		{#snippet children(inPopover)}
			{@const rail = isCollapsed && !inPopover}
			<div class="flex flex-col gap-0.5">
				{#each favoriteManager.current as favorite (`${favorite.kind}:${favorite.path}`)}
					{@const selected = favorite.href !== '#' && page.url.pathname === favorite.href}
					<div class="relative group/fav">
						<!-- An asset's href is `#`: it opens in the database manager instead. -->
						<a
							href={favorite.href}
							title={favorite.path}
							onclick={(e) => {
								if (favorite.kind !== 'asset') return
								e.preventDefault()
								const dbInput = parseDbInputFromAssetSyntax(favorite.path)
								if (dbInput) globalDbManagerDrawer.val?.openDrawer(dbInput)
							}}
							class={twMerge(
								'flex flex-row items-center gap-2 px-2 h-6 rounded text-xs font-normal min-w-0',
								rail ? '' : 'pr-7',
								selected
									? twMerge(sidebarClasses.selectedBg, sidebarClasses.selectedText)
									: 'text-secondary hover:bg-surface-hover'
							)}
						>
							<!-- Kind colors mirror the home-page row icons (RowIcon.svelte). -->
							{#if favorite.kind == 'script'}
								<CodeXml size={14} class="shrink-0 text-blue-500" />
							{:else if favorite.kind == 'flow'}
								<BarsStaggered size={14} class="shrink-0 text-teal-500" />
							{:else if favorite.kind == 'app' || favorite.kind == 'raw_app'}
								<LayoutDashboard size={14} class="shrink-0 text-orange-500" />
							{:else if favorite.kind == 'agent'}
								<Bot size={14} class="shrink-0 text-violet-500" />
							{:else}
								<Table2 size={14} class="shrink-0 text-primary" />
							{/if}
							<span class={rail ? 'sr-only' : 'truncate'}>{favorite.label}</span>
						</a>
						<div
							class={twMerge(
								'absolute right-0.5 top-0 h-full flex items-center opacity-0 group-hover/fav:opacity-100 has-[[aria-expanded=true]]:opacity-100 transition-opacity',
								rail ? 'hidden' : ''
							)}
						>
							<DropdownV2
								items={[
									{
										displayName: 'Remove from favorites',
										icon: Trash2,
										action: () => favoriteManager.unstar(favorite.path, favorite.kind)
									}
								]}
								size="xs"
								fixedHeight={false}
							/>
						</div>
					</div>
				{:else}
					<!-- Kept, unread, in the rail: the rows under it stay where the open sidebar has them. -->
					<p
						class={twMerge(
							'px-2 h-6 flex items-center text-2xs text-hint',
							rail ? 'invisible' : ''
						)}
					>
						Star an item to list it here.
					</p>
				{/each}
			</div>
		{/snippet}
	</SidebarSection>
{/if}
