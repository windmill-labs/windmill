<script lang="ts">
	import { classNames } from '$lib/utils'
	import { Code2, LayoutDashboard } from 'lucide-svelte'
	import { faStar, faBarsStaggered } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'

	import Menu from '../common/menu/Menu.svelte'

	export let isCollapsed: boolean = false
	export let favoriteLinks = [] as {
		label: string
		href: string
		kind: 'script' | 'flow' | 'app'
	}[]
</script>

<Menu placement="bottom-start" let:close>
	<button
		slot="trigger"
		type="button"
		class={classNames(
			'group w-full flex items-center text-white hover:bg-gray-50 hover:text-gray-900 focus:ring-4 focus:outline-none focus:ring-gray-300 px-2 py-2 text-sm font-medium rounded-md h-8 '
		)}
	>
		<Icon
			data={faStar}
			class={classNames('flex-shrink-0 h-4 w-4', isCollapsed ? '-mr-1' : 'mr-2')}
		/>

		{#if !isCollapsed}
			<span class={classNames('whitespace-pre truncate')}>Favorites</span>
		{/if}
	</button>

	<div class="overflow-hidden" role="none">
		{#if !favoriteLinks.length}
			<div class="py-1" role="none">
				<div class="text-gray-600 block px-4 py-2 text-xs" role="menuitem" tabindex="-1">
					Add Scripts/Flows/Apps here by starring them
				</div>
			</div>
		{:else}
			<div class="py-1 w-full max-w-full">
				{#each favoriteLinks ?? [] as favorite (favorite.href)}
					<a href={favorite.href} on:click={close} class="w-full inline-flex flex-row px-4 py-2 hover:bg-gray-100">
						<span class="center-center">
							{#if favorite.kind == 'script'}
								<Code2 size={16} />
							{:else if favorite.kind == 'flow'}
								<Icon data={faBarsStaggered} />
							{:else if favorite.kind == 'app'}
								<LayoutDashboard size={16} />
							{/if}
						</span>
						<span class="text-gray-800 ml-2 grow min-w-0 text-xs truncate">
							{favorite.label}
						</span>
					</a>
				{/each}
			</div>
		{/if}
	</div>
</Menu>
