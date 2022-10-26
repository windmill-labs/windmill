<script lang="ts">
	import type { App } from './types'
	import RunFormComponent from './components/RunFormComponent.svelte'
	import { onMount } from 'svelte'
	import { buildWorld, type World } from './rx'
	import { writable } from 'svelte/store'
	import DisplayComponent from './components/DisplayComponent.svelte'

	export let app: App

	let components: Record<
		string,
		{
			getOutputs: () => string[]
		}
	> = {}
	const worldStore = writable<World | undefined>(undefined)

	onMount(() => {
		$worldStore = buildWorld(
			Object.fromEntries(Object.entries(components).map(([k, v]) => [k, v?.getOutputs()]))
		)

		$worldStore.outputsById['a']['result'].subscribe({
			next: (s) => {
				console.log(s)
			}
		})
	})
</script>

{JSON.stringify($worldStore?.outputsById)}
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
						bind:this={components[component.id]}
						world={$worldStore}
						id={component.id}
					/>
				{:else if component.type === 'displaycomponent'}
					<DisplayComponent
						inputs={component.inputs}
						world={$worldStore}
						bind:this={components[component.id]}
					/>
				{/if}
			</div>
		{/each}
	</div>
{/if}
