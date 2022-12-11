<script lang="ts">
	import { classNames } from '$lib/utils'
	import { faCode, faStar, faTelevision, faBarsStaggered } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'

	import Menu from '../common/menu/Menu.svelte'

	export let isCollapsed: boolean = false
	export let favoriteLinks = [] as {
		label: string
		href: string
		kind: 'script' | 'flow' | 'app'
	}[]
</script>

<Menu placement="bottom-start" let:close noMinW>
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

	<div class="divide-y divide-gray-100  overflow-hidden" role="none">
		<table class="w-full">
			{#each favoriteLinks ?? [] as favorite (favorite.href)}
				<tr>
					<a href={favorite.href} on:click={close} class="inline-flex flex-row px-4 py-2">
						<td class="center-center">
							{#if favorite.kind == 'script'}
								<Icon data={faCode} />
							{:else if favorite.kind == 'flow'}
								<Icon data={faBarsStaggered} />
							{:else if favorite.kind == 'app'}
								<Icon data={faTelevision} />
							{/if}
						</td>
						<td class="text-gray-800 ml-2 grow text-xs center-center truncate">{favorite.label}</td>
					</a>
				</tr>
			{/each}
		</table>
		<div class="py-1" role="none">
			<div class="text-gray-600 block px-4 py-2 text-xs" role="menuitem" tabindex="-1">
				Add Scripts/Flows/Apps here by starring them
			</div>
		</div>
	</div>
</Menu>
