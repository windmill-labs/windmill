<!--
@component
The dot on a pipeline node's top edge: dragged onto an asset, it makes the node's
script run after each write to that asset.
-->
<script lang="ts">
	let { onStart, visible }: { onStart: (e: PointerEvent) => void; visible: boolean } = $props()
</script>

<button
	type="button"
	class="absolute -top-1.5 left-1/2 -translate-x-1/2 z-10 w-3 h-3 rounded-full bg-surface border-2 border-blue-500 cursor-crosshair transition-opacity {visible
		? 'opacity-100'
		: 'opacity-0'}"
	title="Drag onto an asset to run after each write to it"
	aria-label="Drag onto an asset to run after each write to it"
	onpointerdown={(e) => {
		e.stopPropagation()
		e.preventDefault()
		onStart(e)
	}}
	onclick={(e) => e.stopPropagation()}
></button>
