<script lang="ts">
	import { slide } from 'svelte/transition'

	export let title: string
</script>

<section class="pt-1 pb-2 px-1">
	<h1 class="text-sm font-semibold text-left px-2 py-1">
		<slot name="title">
			{title}
		</slot>
	</h1>
	<div transition:slide|local={{ duration: 300 }} class="px-2">
		<slot />
	</div>
</section>
