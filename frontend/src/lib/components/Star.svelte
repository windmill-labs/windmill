<script lang="ts">
	import { preventDefault } from 'svelte/legacy'

	import { FavoriteService } from '$lib/gen'
	import { starStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Star, StarOff } from 'lucide-svelte'

	interface Props {
		path: string
		kind: 'flow' | 'app' | 'script' | 'raw_app'
		starred?: boolean
		workspace_id: string
		onStarred?: (starred: boolean) => void
	}

	let { path, kind, starred = false, workspace_id, onStarred }: Props = $props()

	let buttonHover = $state(false)

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
		onStarred?.(!starred)
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
