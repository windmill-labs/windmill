<script lang="ts">
	import { FavoriteService } from '$lib/gen'
	import { starStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { createEventDispatcher } from 'svelte'
	import { Star, StarOff } from 'lucide-svelte'

	export let path: string
	export let kind: 'flow' | 'app' | 'script' | 'raw_app'
	export let starred = false
	export let workspace_id: string

	let buttonHover = false

	async function onClick() {
		buttonHover = false
		if (starred) {
			await FavoriteService.unstar({
				workspace: workspace_id,
				requestBody: { path, favorite_kind: kind }
			})
			sendUserToast('Unstarred')
			$starStore = $starStore + 1
		} else {
			await FavoriteService.star({
				workspace: workspace_id,
				requestBody: { path, favorite_kind: kind }
			})
			sendUserToast('Marked as favorite, it will appear first')
			$starStore = $starStore + 1
		}
		dispatch('starred', !starred)
	}
	const dispatch = createEventDispatcher()
</script>

<button
	on:click|preventDefault={onClick}
	on:mouseenter={() => (buttonHover = true)}
	on:mouseleave={() => (buttonHover = false)}
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
