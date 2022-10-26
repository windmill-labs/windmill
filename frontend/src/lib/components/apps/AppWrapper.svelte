<script lang="ts">
	import type { App } from './types'
	import RunFormComponent from './components/RunFormComponent.svelte'
	import { onMount } from 'svelte'
	import { buildWorld, type World } from './rx'
	import { writable } from 'svelte/store'
	import DisplayComponent from './components/DisplayComponent.svelte'

	export let app: App

	let components: Record<string, string[]> = {}
	const worldStore = writable<World | undefined>(undefined)

	onMount(() => {
		$worldStore = buildWorld(components)

		$worldStore.outputsById['a']['result'].subscribe({
			next: (s) => {
				console.log(s)
			}
		})
	})
</script>

{#if app}
	<h2 class="mb-4">{app.title}</h2>
	<div class="flex flex-col w-full space-y-2">
		{#each app.components as component (component.id)}
			<div class="border p-4">
				{#if component.type === 'runformcomponent'}
					<RunFormComponent
						{...component}
						bind:staticOutputs={components[component.id]}
						world={$worldStore}
					/>
				{:else if component.type === 'displaycomponent'}
					<DisplayComponent
						{...component}
						world={$worldStore}
						bind:staticOutputs={components[component.id]}
					/>
				{/if}
			</div>
		{/each}
	</div>
{/if}
