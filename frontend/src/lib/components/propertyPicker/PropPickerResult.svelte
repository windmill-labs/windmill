<script lang="ts">
	import { createEventDispatcher } from 'svelte'

	import ObjectViewer from './ObjectViewer.svelte'

	export let result: any
</script>

<div class="w-full">
	<span class="font-bold text-sm">Result</span>
	<div class="overflow-y-auto mb-2 w-full">
		<ObjectViewer json={{ result }} on:select />
	</div>
</div>
