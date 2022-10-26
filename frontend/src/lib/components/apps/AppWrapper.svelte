<script lang="ts">
	import type { App } from './types'
	import RunFormComponent from './components/RunFormComponent.svelte'

	export let app: App
</script>

{#if app}
	<h2 class="mb-4">{app.title}</h2>
	<div class="flex flex-col w-full space-y-2">
		{#each app.components as component, index (component.id)}
			<div class="border p-4">
				{#if component.type === 'runformcomponent'}
					<RunFormComponent
						runType={component.runType}
						args={component.inputs.runInputs}
						path={component.path}
						hidden={component.params.hidden}
					/>
				{:else if component.type === 'displaycomponent'}
					<div>display</div>
				{/if}
			</div>
		{/each}
	</div>
{/if}
