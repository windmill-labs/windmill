<script lang="ts">
	import { preventDefault } from 'svelte/legacy'

	import { Star, StarOff } from 'lucide-svelte'
	import { favoriteManager } from './sidebar/FavoriteMenu.svelte'

	interface Props {
		path: string
		kind: 'flow' | 'app' | 'script' | 'raw_app'
		workspaceId?: string
	}

	let { path, kind, workspaceId }: Props = $props()

	let buttonHover = $state(false)
	let starred = $derived(favoriteManager.isStarred(path, kind))

	async function onClick() {
		buttonHover = false
		if (starred) favoriteManager.unstar(path, kind, workspaceId)
		else favoriteManager.star(path, kind, workspaceId)
	}
</script>

<button
	onclick={preventDefault(onClick)}
	onmouseenter={() => (buttonHover = true)}
	onmouseleave={() => (buttonHover = false)}
	class="p-2"
>
	{#if starred}
		{#if buttonHover}
			<StarOff size={16} fill="currentcolor" />
		{:else}
			<Star size={16} fill="currentcolor" />
		{/if}
	{:else}
		<Star
			class={!buttonHover ? 'opacity-60' : ''}
			size={16}
			fill={buttonHover ? 'currentcolor' : 'none'}
		/>
	{/if}
</button>
