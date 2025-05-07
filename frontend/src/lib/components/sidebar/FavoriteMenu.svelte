<script lang="ts">
	import { CodeXml, LayoutDashboard, Star } from 'lucide-svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import { Menu, MenuItem } from '$lib/components/meltComponents'
	import MenuButton from '$lib/components/sidebar/MenuButton.svelte'
	import type { MenubarBuilders } from '@melt-ui/svelte'

	export let lightMode: boolean = false

	export let isCollapsed: boolean = false
	export let favoriteLinks = [] as {
		label: string
		href: string
		kind: 'script' | 'flow' | 'app' | 'raw_app'
	}[]
	export let createMenu: MenubarBuilders['createMenu']
</script>

<Menu {createMenu} let:item usePointerDownOutside>
	<svelte:fragment slot="trigger" let:trigger>
		<MenuButton
			class="!text-xs"
			icon={Star}
			label={'Favorites'}
			{isCollapsed}
			{lightMode}
			{trigger}
		/>
	</svelte:fragment>

	<div class="overflow-hidden" role="none">
		{#if !favoriteLinks.length}
			<div class="py-1" role="none">
				<div class="text-secondary block px-4 py-2 text-xs" role="menuitem" tabindex="-1">
					Star items first
				</div>
			</div>
		{:else}
			<div class="py-1 w-full max-w-full">
				{#each favoriteLinks ?? [] as favorite (favorite.href)}
					<MenuItem
						href={favorite.href}
						{item}
						class="w-full inline-flex flex-row px-4 py-2 data-[highlighted]:bg-surface-hover"
					>
						<span class="center-center">
							{#if favorite.kind == 'script'}
								<CodeXml size={16} />
							{:else if favorite.kind == 'flow'}
								<BarsStaggered size={16} />
							{:else if favorite.kind == 'app' || favorite.kind == 'raw_app'}
								<LayoutDashboard size={16} />
							{/if}
						</span>
						<span class="text-primary ml-2 grow min-w-0 text-xs truncate">
							{favorite.label}
						</span>
					</MenuItem>
				{/each}
			</div>
		{/if}
	</div>
</Menu>
