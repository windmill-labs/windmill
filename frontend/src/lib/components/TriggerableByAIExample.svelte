<script lang="ts">
	import TriggerableByAI from './TriggerableByAI.svelte'

	let isVisible = $state(true)

	function toggleVisibility() {
		isVisible = !isVisible
	}
</script>

{#if isVisible}
	<TriggerableByAI
		id="example-component"
		description="This is an example component that can be triggered by AI"
		onTrigger={(id: string) => {
			console.log('Triggering example component', id)
		}}
	>
		<div class="p-4 border rounded">
			<!-- Your actual component content goes here -->
			<h3 class="text-lg font-medium">AI Triggerable Component</h3>
			<p
				>This component will register itself with the AI system on mount and unregister on unmount.</p
			>
			<button
				class="mt-2 px-3 py-1 bg-blue-500 text-white rounded hover:bg-blue-600"
				onclick={toggleVisibility}
			>
				Hide Component
			</button>
		</div>
	</TriggerableByAI>
{:else}
	<button
		class="px-3 py-1 bg-blue-500 text-white rounded hover:bg-blue-600"
		onclick={toggleVisibility}
	>
		Show Component
	</button>
{/if}
