<script lang="ts">
	import { preventDefault } from 'svelte/legacy'

	import { Star, StarOff } from 'lucide-svelte'
	import { favoriteManager, type FavoriteKind } from './sidebar/FavoriteMenu.svelte'

	interface Props {
		path: string
		kind: FavoriteKind
		summary?: string
		workspaceId?: string
		size?: number
		/** Starred items show in yellow. */
		yellowWhenStarred?: boolean
		/** Hidden until the surrounding `group/row` is hovered or focused, unless starred. */
		revealOnRowHover?: boolean
	}

	let {
		path,
		kind,
		workspaceId,
		summary,
		size = 16,
		yellowWhenStarred = false,
		revealOnRowHover = false
	}: Props = $props()

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
	class={[
		'p-1',
		starred && yellowWhenStarred && 'text-yellow-500',
		!starred &&
			revealOnRowHover &&
			'invisible group-hover/row:visible group-focus-within/row:visible group-data-[row-keyboard-selected=true]/row:visible'
	]}
>
	{#if starred}
		{#if buttonHover}
			<StarOff {size} fill="currentcolor" />
		{:else}
			<Star {size} fill="currentcolor" />
		{/if}
	{:else}
		<Star
			class={!buttonHover ? 'opacity-60' : ''}
			{size}
			fill={buttonHover ? 'currentcolor' : 'none'}
		/>
	{/if}
</button>
