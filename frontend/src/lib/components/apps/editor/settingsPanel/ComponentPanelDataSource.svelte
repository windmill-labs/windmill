<script lang="ts">
	import type { AppComponent } from '../component'

	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import PlotlyRichEditor from './PlotlyRichEditor.svelte'
	import { onMount } from 'svelte'

	export let component: AppComponent

	let selected = 'ui-editor'

	onMount(() => {
		if (component.type === 'plotlycomponentv2' && component.datasets !== undefined) {
			selected = 'ui-editor'
		} else {
			selected = 'json'
		}
	})
</script>

{#if component.type === 'plotlycomponentv2'}
	<div class="p-2">
		<ToggleButtonGroup
			bind:selected
			on:selected={() => {
				if (selected === 'ui-editor' && component.type === 'plotlycomponentv2') {
					component.datasets = {
						type: 'static',
						fieldType: 'array',
						subFieldType: 'plotly',
						value: [
							{
								value: {
									type: 'static',
									fieldType: 'array',
									subFieldType: 'number',
									value: [1, 2, 3, 4]
								},
								name: 'Dataset 1',
								aggregation_method: 'sum',
								type: 'bar',
								toolip: 'This is an example',
								color: '#C8A2C8'
							}
						]
					}

					component.xData = {
						type: 'evalv2',
						fieldType: 'array',
						expr: '[1, 2, 3, 4]',
						connections: []
					}
				}

				if (selected === 'json' && component.type === 'plotlycomponentv2') {
					component.datasets = undefined
					component.xData = undefined
				}
			}}
		>
			<ToggleButton
				value="ui-editor"
				label="UI Editor"
				tooltip="Use the UI editor to quickly create a plotly chart."
			/>
			<ToggleButton
				value="json"
				label="JSON"
				tooltip="Switch to JSON mode for complete customization of Plotly settings."
			/>
		</ToggleButtonGroup>
	</div>

	{#if selected === 'ui-editor'}
		<PlotlyRichEditor
			id={component.id}
			bind:datasets={component.datasets}
			bind:xData={component.xData}
		/>
	{:else}
		<slot />
	{/if}
{:else}
	<slot />
{/if}
