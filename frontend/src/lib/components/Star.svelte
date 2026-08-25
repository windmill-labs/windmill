<script lang="ts">
	import { preventDefault } from 'svelte/legacy'

	import { Star, StarOff } from 'lucide-svelte'
	import { favoriteManager, type FavoriteKind } from './sidebar/FavoriteMenu.svelte'

	interface Props {
		path: string
		kind: FavoriteKind
		summary?: string
		workspaceId?: string
	}

	let { path, kind, workspaceId, summary }: Props = $props()

	let buttonHover = $state(false)
	let starred = $derived(favoriteManager.isStarred(path, kind))

	async function onClick() {
		buttonHover = false
		if (starred) favoriteManager.unstar(path, kind, workspaceId)
		else favoriteManager.star(path, kind, workspaceId, summary)
	}
</script>

<button
	onclick={preventDefault(onClick)}
	onmouseenter={() => (buttonHover = true)}
	onmouseleave={() => (buttonHover = false)}
	class="p-1"
>
	<!-- Never filled: a favourite reads as yellow, not as a solid blob, which keeps
	     the icon the same weight whether or not it is starred. -->
	{#if starred}
		{#if buttonHover}
			<StarOff size={16} fill="none" class="text-yellow-500" />
		{:else}
			<Star size={16} fill="none" class="text-yellow-500" />
		{/if}
	{:else}
		<Star size={16} fill="none" class={buttonHover ? '' : 'opacity-60'} />
	{/if}
</button>
