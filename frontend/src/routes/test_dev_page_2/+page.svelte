<script lang="ts">
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Button from '$lib/components/common/button/Button.svelte'

	let sampleJson = {
		name: 'Test Object',
		description: 'A sample JSON object for testing',
		properties: {
			number: 42,
			boolean: true,
			array: [1, 2, 3, 'four', 'five'],
			nested: {
				key1: 'value1',
				key2: 'value2'
			}
		},
		timestamp: new Date().toISOString()
	}

	function handleFileSelect(event: Event) {
		const input = event.target as HTMLInputElement
		const file = input.files?.[0]

		if (file) {
			const reader = new FileReader()
			reader.onload = (e) => {
				try {
					sampleJson = JSON.parse(e.target?.result as string)
				} catch (error) {
					alert('Error parsing JSON file: ' + error)
				}
			}
			reader.readAsText(file)
		}
	}

	const usePopover = false
</script>

<div class="container">
	<div class="content">
		<input type="file" accept=".json" on:change={handleFileSelect} class="file-input" />
		{#if usePopover}
			<Popover contentClasses="p-4">
				<svelte:fragment slot="trigger">
					<Button nonCaptureEvent>Open</Button>
				</svelte:fragment>
				<svelte:fragment slot="content">
					<ObjectViewer json={sampleJson} />
				</svelte:fragment>
			</Popover>
		{:else}
			<ObjectViewer json={sampleJson} />
		{/if}
	</div>
</div>

<style>
	.container {
		display: flex;
		justify-content: center;
		align-items: center;
		min-height: 100vh;
		padding: 2rem;
	}

	.content {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		width: 100%;
		max-width: 800px;
	}

	.file-input {
		margin-bottom: 1rem;
	}
</style>
