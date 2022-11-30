<script lang="ts">
	import { FavoriteService } from '$lib/gen'
	import { starStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/utils'
	import { faStar } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'
	import { Star } from 'svelte-lucide'

	export let path: string
	export let kind: 'flow' | 'app' | 'script'
	export let starred = false
	export let workspace_id: string

	async function onClick() {
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

<button on:click|preventDefault={onClick} class="mx-1">
	{#if starred}
		<div>
			<Icon data={faStar} class="hover:text-gray-300" scale={1.1} />
		</div>
	{:else}
		<Star size="18px" class="hover:bg-gray-200" />
	{/if}
</button>
