<script lang="ts">
	import Button from './common/button/Button.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import Section from './Section.svelte'
	import SimpleEditor from './SimpleEditor.svelte'
	import Subsection from './Subsection.svelte'
	import {
		generateHttpTriggerFromCurl,
		generateHttpTriggerFromOpenApi,
		type Source
	} from './triggers/http/utils'

	type Props = {}

	let {}: Props = $props()

	let selected: Source = $state('OpenApiFile')
	let curlCommand = $state('')
	let openApiSpec = $state('')
	let isGeneratingHttpRoutes = $state(false)
	let filePath = $state('')
</script>

<Section label="Http routes generator">
	<Subsection>
		<ToggleButtonGroup bind:selected let:item>
			<ToggleButton
				tooltip="Upload an OpenAPI file to generate HTTP routes automatically"
				showTooltipIcon
				label="Generate from OpenAPI file"
				value="OpenApiFile"
				{item}
			/>
			<ToggleButton
				tooltip="Paste an OpenAPI JSON/YAML spec directly to generate HTTP routes"
				showTooltipIcon
				label="Paste OpenAPI spec"
				value="OpenApi"
				{item}
			/>
			<ToggleButton
				tooltip="Enter a curl command to parse and generate HTTP routes from it"
				showTooltipIcon
				label="Generate from curl command"
				value="curl"
				{item}
			/>
		</ToggleButtonGroup>

		<div class="flex flex-col gap-2 mt-2">
			{#if selected === 'OpenApiFile'}{:else if selected === 'OpenApi'}
				<div class="h-96">
					<SimpleEditor class="h-full" lang={'yaml'} bind:code={openApiSpec} />
				</div>
			{:else if selected === 'curl'}
				<div class="h-96">
					<SimpleEditor class="h-full" lang={'bash'} bind:code={curlCommand} />
				</div>
			{/if}
			<Button
				spacingSize="sm"
				size="xs"
				btnClasses="h-8"
				loading={isGeneratingHttpRoutes}
				on:click={() => {
					if (selected === 'curl') {
						generateHttpTriggerFromCurl(curlCommand)
						return
					}
					const data = selected === 'OpenApiFile' ? filePath : openApiSpec
					generateHttpTriggerFromOpenApi(data, selected)
				}}
				color="light"
				variant="border">Generate Http routes</Button
			>
		</div>
	</Subsection>
</Section>
