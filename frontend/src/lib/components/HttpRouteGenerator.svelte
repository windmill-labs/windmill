<script lang="ts">
	import { HttpTriggerService, type NewHttpTrigger } from '$lib/gen'
	import { Pen } from 'lucide-svelte'
	import Button from './common/button/Button.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import Section from './Section.svelte'
	import SimpleEditor from './SimpleEditor.svelte'
	import Subsection from './Subsection.svelte'
	import RouteEditor from './triggers/http/RouteEditor.svelte'
	import { generateHttpTriggerFromOpenApi, type Source } from './triggers/http/utils'
	import { isCloudHosted } from '$lib/cloud'
	import { workspaceStore } from '$lib/stores'
	import FileInput from './common/fileInput/FileInput.svelte'
	import { emptyStringTrimmed, sendUserToast } from '$lib/utils'
	import FolderPicker from './FolderPicker.svelte'

	type Props = {
		closeFn: () => Promise<void>
	}

	let { closeFn }: Props = $props()

	let routeEditor: RouteEditor

	let selected: Source = $state('OpenApiFile')

	let openApiUrl = $state('')
	let openApiFile = $state('')

	let openApiRawEditorValue = $state('')
	let openApiFileEditorValue = $state('')
	let openApiUrlEditorValue = $state('')
	let isGeneratingHttpRoutes = $state(false)
	let httpTriggers: NewHttpTrigger[] = $state([])
	let isFetchingOpenApiSpec = $state(false)
	let folderName: string = $state('')
	let callback: ((newHttpTrigger: NewHttpTrigger) => void) | undefined = $state(undefined)

	let lang: 'yaml' | 'json' = $derived.by(() => {
		if (code.trimStart().startsWith('{')) {
			return 'json'
		}

		return 'yaml'
	})
	let acceptedFileTypes: string[] = ['.json', '.yaml']
	let isCreating = $state(false)
	function openRouteEditor(path: string, newHttpTrigger: NewHttpTrigger) {
		routeEditor.openNew(false, path, newHttpTrigger)
	}

	let code: string = $state('')
	let isValidUrl = $derived.by(() => {
		try {
			const parsed = new URL(openApiUrl)
			return /^https?:/.test(parsed.protocol)
		} catch (err) {
			return false
		}
	})

	async function fetchOpenApiConfig() {
		try {
			isFetchingOpenApiSpec = true
			const response = await fetch(openApiUrl)

			const data = await response.text()
			openApiUrlEditorValue = data
			code = data
		} catch (error) {
			sendUserToast(error.body, true)
		} finally {
			isFetchingOpenApiSpec = false
		}
	}

	async function saveHttpTrigger() {
		try {
			isCreating = true
			const message = await HttpTriggerService.createManyHttpTrigger({
				workspace: $workspaceStore!,
				requestBody: httpTriggers
			})
			sendUserToast(message)
			await closeFn()
		} catch (error) {
			sendUserToast(error.body, true)
		} finally {
			isCreating = false
		}
	}

	async function generateHttpTrigger() {
		if (emptyStringTrimmed(code)) {
			sendUserToast('Please enter an openapi spec first', true)
			return
		}
		try {
			httpTriggers = await generateHttpTriggerFromOpenApi(code, folderName)
		} catch (error) {
			sendUserToast(error, true)
		}
	}
</script>

<RouteEditor updateHttpTrigger={callback} bind:this={routeEditor} />

<Section label="Http routes generator">
	<div class="flex flex-col gap-2">
		<Subsection label="Pick a folder">
			<FolderPicker bind:folderName />
		</Subsection>

		{#if !emptyStringTrimmed(folderName)}
			<Subsection>
				<ToggleButtonGroup
					on:selected={({ detail }) => {
						const type = detail as Source
						if (type === 'OpenApi') {
							code = openApiRawEditorValue
						} else if (type === 'OpenApiFile') {
							code = openApiFileEditorValue
						} else {
							code = openApiUrlEditorValue
						}
					}}
					bind:selected
					let:item
				>
					<ToggleButton
						tooltip="Upload an OpenAPI file to generate HTTP trigger(s) automatically"
						showTooltipIcon
						label="Generate from OpenAPI file"
						value="OpenApiFile"
						{item}
					/>
					<ToggleButton
						tooltip="Paste an OpenAPI JSON/YAML spec directly to generate HTTP trigger(s)"
						showTooltipIcon
						label="Paste OpenAPI spec"
						value="OpenApi"
						{item}
					/>
					<ToggleButton
						tooltip="Enter an OpenApi URL to generate an HTTP routes from it"
						showTooltipIcon
						label="Generate from OpenApi URL"
						value="OpenApiURL"
						{item}
					/>
				</ToggleButtonGroup>

				<div class="flex flex-col gap-2 mt-2">
					{#if selected === 'OpenApiFile'}
						<FileInput
							accept={acceptedFileTypes.join(',')}
							multiple={false}
							convertTo={'text'}
							iconSize={24}
							returnFileNames={true}
							class="text-sm py-4"
							on:change={async ({ detail }) => {
								if (detail && detail.length > 0) {
									code = detail[0].data
									openApiFileEditorValue = code
									openApiFile = detail[0].name as string
									if (openApiFile.endsWith('.json')) {
										lang = 'json'
										return
									}
									lang = 'yaml'
									return
								}
								openApiFile = ''
							}}
						/>
					{:else if selected === 'OpenApi'}
						<ToggleButtonGroup bind:selected={lang} let:item>
							<ToggleButton value="yaml" label="yaml" {item} />
							<ToggleButton value="json" label="json" {item} />
						</ToggleButtonGroup>
					{:else if selected === 'OpenApiURL'}
						<div class="flex flex-row gap-1">
							<input type="text" placeholder="OpenApi URL" bind:value={openApiUrl} />
							<Button
								color="light"
								spacingSize="sm"
								size="xs"
								loading={isFetchingOpenApiSpec}
								disabled={!isValidUrl}
								on:click={fetchOpenApiConfig}>Fetch</Button
							>
						</div>
					{/if}
					{#if selected === 'OpenApi' || (selected === 'OpenApiFile' && !emptyStringTrimmed(openApiFile)) || (selected === 'OpenApiURL' && !emptyStringTrimmed(openApiUrl))}
						<div class="h-96">
							<SimpleEditor class="h-full" {lang} bind:code />
						</div>
					{/if}
					<Button
						spacingSize="sm"
						size="xs"
						btnClasses="h-8"
						loading={isGeneratingHttpRoutes}
						on:click={generateHttpTrigger}
						color="light"
						variant="border">Generate Http trigger(s)</Button
					>
				</div>
			</Subsection>
			<Subsection>
				<div class="flex flex-col gap-1 mb-2">
					{#each httpTriggers as httpTrigger, index}
						<div
							class="hover:bg-surface-hover w-full items-center px-4 py-2 gap-4 first-of-type:!border-t-0
				first-of-type:rounded-t-md last-of-type:rounded-b-md flex justify-between mt-2"
						>
							<div>
								<div class="text-primary">
									{httpTrigger.http_method.toUpperCase()}
									{isCloudHosted() || httpTrigger.workspaced_route
										? $workspaceStore! + '/' + httpTrigger.route_path
										: httpTrigger.route_path}
								</div>
								<div class="text-secondary text-xs truncate text-left font-light">
									{httpTrigger.path}
								</div>
							</div>

							<div class="flex gap-2 items-center justify-end">
								<Button
									on:click={() => {
										callback = (newHttpTrigger) => {
											console.log({ newHttpTrigger })
											httpTrigger[index] = newHttpTrigger
										}
										openRouteEditor(httpTrigger.path, httpTrigger)
									}}
									size="xs"
									startIcon={{ icon: Pen }}
									color="gray"
								>
									Edit
								</Button>
							</div>
						</div>
					{/each}
				</div>
			</Subsection>
			<Button
				spacingSize="sm"
				size="xs"
				btnClasses="h-8"
				loading={isCreating}
				disabled={httpTriggers.length === 0}
				on:click={saveHttpTrigger}
				color="light"
				variant="border"
				>Save HTTP trigger(s)
			</Button>
		{/if}
	</div>
</Section>
