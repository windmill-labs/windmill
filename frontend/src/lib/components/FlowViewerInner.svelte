<script lang="ts">
	import Highlight from 'svelte-highlight'
	import json from 'svelte-highlight/languages/json'
	import type { FlowValue } from '$lib/gen'
	import { Tab, Tabs, Button } from './common'
	import { copyToClipboard } from '../utils'

	import { ArrowDown, Clipboard } from 'lucide-svelte'
	import YAML from 'yaml'
	import { yaml } from 'svelte-highlight/languages'
	import HighlightTheme from './HighlightTheme.svelte'
	import { filteredContentForExport } from './flows/utils'

	interface Props {
		flow: {
			summary: string
			description?: string
			value: FlowValue
			schema?: any
		}
	}

	let { flow }: Props = $props()

	let flowFiltered = $derived(filteredContentForExport(flow))

	let rawType: 'json' | 'yaml' = $state('yaml')

	function trimStringToLines(inputString: string, maxLines: number = 100): string {
		const lines = inputString?.split('\n') ?? []
		const linesToKeep = lines.slice(0, maxLines)

		return linesToKeep.join('\n')
	}

	let code: string = $state('')

	function computeCode() {
		const str =
			rawType === 'json' ? JSON.stringify(flowFiltered, null, 4) : YAML.stringify(flowFiltered)

		const numberOfLines = str.split('\n').length

		if (numberOfLines > maxLines) {
			shouldDisplayLoadMore = true
		}

		code = str
	}

	let shouldDisplayLoadMore = $state(false)

	$effect(() => {
		flowFiltered && rawType && computeCode()
	})

	let maxLines = $state(100)
</script>

<HighlightTheme />

<div>
	<Tabs
		bind:selected={rawType}
		on:selected={() => {
			maxLines = 100
		}}
	>
		<Tab value="yaml">YAML</Tab>
		<Tab value="json">JSON</Tab>
		{#snippet content()}
			<div class="relative pt-2">
				<Button
					on:click={() =>
						copyToClipboard(
							rawType === 'yaml'
								? YAML.stringify(flowFiltered)
								: JSON.stringify(flowFiltered, null, 4)
						)}
					color="light"
					variant="border"
					size="xs"
					startIcon={{ icon: Clipboard }}
					btnClasses="absolute top-2 right-2 w-min z-20"
					iconOnly
				/>

				<div class={shouldDisplayLoadMore ? 'code-container' : ''}>
					<Highlight
						class="overflow-auto px-1"
						language={rawType === 'yaml' ? yaml : json}
						code={trimStringToLines(code, maxLines)}
					/>
				</div>
				{#if shouldDisplayLoadMore}
					<Button
						on:click={() => {
							maxLines += 500

							// If the code is less than the max lines, we don't need to show the button
							if (maxLines >= code?.split('\n').length) {
								shouldDisplayLoadMore = false
							}
						}}
						color="light"
						size="xs"
						btnClasses="mb-2 mx-2"
						startIcon={{ icon: ArrowDown }}
					>
						Show more
					</Button>
				{/if}
			</div>
		{/snippet}
	</Tabs>
</div>

<style>
	.code-container {
		position: relative;
		overflow: hidden;
	}
	.code-container::after {
		content: '';
		position: absolute;
		bottom: 0;
		left: 0;
		right: 0;
		height: 100px;
		background: linear-gradient(to bottom, rgba(255, 255, 255, 0), rgb(var(--color-surface)));
		pointer-events: none;
	}
</style>
