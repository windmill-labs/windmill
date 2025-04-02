<script lang="ts">
	import { initializeMode } from 'monaco-graphql/esm/initializeMode.js'
	import DynamicContainer from '$lib/components/DynamicContainer.svelte'
	initializeMode()
</script>

<div class="p-8">
	<DynamicContainer />
</div>
