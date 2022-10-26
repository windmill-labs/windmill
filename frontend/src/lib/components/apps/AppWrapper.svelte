<script lang="ts">
	import DbIcon from '../icons/DbIcon.svelte'
	import type { App } from './types'

	export let app: App
</script>

{#if app}
	<h2>{app.title}</h2>
	<div class="flex flex-col w-full space-y-2">
		{#each app.components as component, index (component.id)}
			{#if component.type === 'runformcomponent'}
				<div>form</div>
			{:else if component.type === 'displaycomponent'}
				<div>display</div>
			{/if}
		{/each}
	</div>
{/if}
