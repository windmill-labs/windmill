<script lang="ts">
	import { Code2, LayoutDashboard, Star } from 'lucide-svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'

	import Menu from '../common/menu/MenuV2.svelte'
	import MenuButton from './MenuButton.svelte'
	import { MenuItem } from '@rgossiaux/svelte-headlessui'

	export let isCollapsed: boolean = false
	export let favoriteLinks = [] as {
		label: string
		href: string
		kind: 'script' | 'flow' | 'app' | 'raw_app'
	}[]
</script>

<Menu>
	<div slot="trigger">
		<MenuButton class="!text-xs" icon={Star} label={'Favorites'} {isCollapsed} />
	</div>

	<div class="overflow-hidden" role="none">
		{#if !favoriteLinks.length}
			<div class="py-1" role="none">
				<div class="text-secondary block px-4 py-2 text-xs" role="menuitem" tabindex="-1">
					Add Scripts/Flows/Apps here by starring them
				</div>
			</div>
		{:else}
			<div class="py-1 w-full max-w-full">
				{#each favoriteLinks ?? [] as favorite (favorite.href)}
					<MenuItem>
						<a
							href={favorite.href}
							on:click={close}
							class="w-full inline-flex flex-row px-4 py-2 hover:bg-surface-hover"
						>
							<span class="center-center">
								{#if favorite.kind == 'script'}
									<Code2 size={16} />
								{:else if favorite.kind == 'flow'}
									<BarsStaggered size={16} />
								{:else if favorite.kind == 'app' || favorite.kind == 'raw_app'}
									<LayoutDashboard size={16} />
								{/if}
							</span>
							<span class="text-primary ml-2 grow min-w-0 text-xs truncate">
								{favorite.label}
							</span>
						</a>
					</MenuItem>
				{/each}
			</div>
		{/if}
	</div>
</Menu>
