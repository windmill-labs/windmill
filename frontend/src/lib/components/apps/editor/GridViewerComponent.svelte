<script lang="ts">
	export let width: number
	export let height: number
	export let left: number
	export let top: number

	export let onTop: boolean
</script>

<div
	class="svlt-grid-item"
	style="width: {width}px; height:{height}px; {onTop ? 'z-index: 100;' : ''}
  {`transition: transform 0.1s, opacity 0.1s; transform: translate(${left}px, ${top}px); `}"
>
	<slot />
</div>

<style>
	.svlt-grid-item {
		touch-action: none;
		position: absolute;
		will-change: auto;
		backface-visibility: hidden;
		-webkit-backface-visibility: hidden;
	}
</style>
