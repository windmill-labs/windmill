<script lang="ts">
	import { Star, StarOff } from 'lucide-svelte'

	interface Props {
		starred: boolean
		onToggle: () => void
		size?: number
		class?: string
	}

	let { starred, onToggle, size = 13, class: className = '' }: Props = $props()

	let hover = $state(false)
</script>

<!-- The real Star's look and states; it toggles the playground's data instead of the server's. -->
<button
	type="button"
	onclick={(e) => {
		e.preventDefault()
		onToggle()
	}}
	onmouseenter={() => (hover = true)}
	onmouseleave={() => (hover = false)}
	class={className}
	data-starred={starred || undefined}
	title={starred ? 'Remove from favorites' : 'Add to favorites'}
>
	{#if starred}
		{#if hover}
			<StarOff {size} fill="currentcolor" />
		{:else}
			<Star {size} fill="currentcolor" />
		{/if}
	{:else}
		<Star class={hover ? '' : 'opacity-60'} {size} fill={hover ? 'currentcolor' : 'none'} />
	{/if}
</button>
