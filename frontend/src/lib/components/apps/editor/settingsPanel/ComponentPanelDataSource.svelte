<script lang="ts">
	import type { AppComponent } from '../component'

	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import PlotlyRichEditor from './PlotlyRichEditor.svelte'

	export let component: AppComponent

	let selected = 'ui-editor'
</script>

{#if component.type === 'plotlycomponentv2'}
	<div class="p-2">
		<ToggleButtonGroup bind:selected>
			<ToggleButton value="ui-editor" label="UI Editor" />
			<ToggleButton value="json" label="JSON" />
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
