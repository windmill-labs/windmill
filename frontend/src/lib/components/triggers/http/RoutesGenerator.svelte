<script lang="ts">
	import { HttpTriggerService, type EditHttpTrigger, type HttpTrigger, type NewHttpTrigger } from '$lib/gen'
	import { Pen, Save } from 'lucide-svelte'
	import Button from '../../common/button/Button.svelte'
	import ToggleButton from '../../common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '../../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import Section from '../../Section.svelte'
	import SimpleEditor from '../../SimpleEditor.svelte'
	import Subsection from '../../Subsection.svelte'
	import RouteEditor from './RouteEditor.svelte'
	import { generateHttpTriggerFromOpenApi, type Source } from './utils'
	import { isCloudHosted } from '$lib/cloud'
	import { usedTriggerKinds, workspaceStore } from '$lib/stores'
	import FileInput from '../../common/fileInput/FileInput.svelte'
	import { emptyStringTrimmed, sendUserToast } from '$lib/utils'
	import FolderPicker from '../../FolderPicker.svelte'
	import Required from '$lib/components/Required.svelte'
	import { Drawer, DrawerContent } from '$lib/components/common'
	import { get } from 'svelte/store'

	type Props = {
		closeFn: () => Promise<void>
	}

	let { closeFn }: Props = $props()

	let routeEditor: RouteEditor
	let routesGenerator: Drawer

	let selected: Source = $state('OpenAPI')
	let openApiUrl = $state('')
	let openApiFile = $state('')
	let openApiRawEditorValue = $state('')
	let openApiFileEditorValue = $state('')
	let openApiUrlEditorValue = $state('')
	let isGeneratingHttpRoutes = $state(false)
	let httpTriggers: NewHttpTrigger[] = $state([])
	let isFetchingOpenApiSpec = $state(false)
	let folderName: string = $state('')
	let forceRerender = $state(false)
	let code: string = $state('')
	let isCreating = $state(false)
	let acceptedFileTypes: string[] = ['.json', '.yaml']

	let callback: ((cfg: HttpTrigger | EditHttpTrigger) => void) | undefined = $state(undefined)

	let lang: 'yaml' | 'json' = $derived.by(() => {
		if (code.trimStart().startsWith('{')) {
			return 'json'
		}

		return 'yaml'
	})

	function openRouteEditor(_path: string, newHttpTrigger: NewHttpTrigger) {
		routeEditor.openNew(false, '', newHttpTrigger)
	}

	let isValidUrl = $derived.by(() => {
		try {
			const parsed = new URL(openApiUrl)
			return /^https?:/.test(parsed.protocol)
		} catch (err) {
			return false
		}
	})

	export function openDrawer() {
		routesGenerator.openDrawer()
	}

	async function fetchOpenApiConfig() {
		try {
			isFetchingOpenApiSpec = true
			const response = await fetch(openApiUrl)

			const data = await response.text()
			openApiUrlEditorValue = data
			code = data
			forceRerender = !forceRerender
		} catch (error) {
			sendUserToast(error.message || error, true)
		} finally {
			isFetchingOpenApiSpec = false
		}
	}

	async function saveHttpTrigger() {
		try {
			isCreating = true
			const message = await HttpTriggerService.createHttpTriggers({
				workspace: $workspaceStore!,
				requestBody: httpTriggers
			})
			sendUserToast(message)
			await closeFn()
			routesGenerator.closeDrawer()
			if (!get(usedTriggerKinds).includes('http')) {
				usedTriggerKinds.update((t) => [...t, 'http'])
			}
			//reset
			selected = 'OpenAPI'
			openApiUrl = ''
			openApiFile = ''
			openApiRawEditorValue = ''
			openApiFileEditorValue = ''
			openApiUrlEditorValue = ''
			httpTriggers = []
			folderName = ''
			code = ''
		} catch (error) {
			sendUserToast(error.body, true)
		} finally {
			isCreating = false
		}
	}

	async function generateHttpTrigger() {
		try {
			isGeneratingHttpRoutes = true
			httpTriggers = await generateHttpTriggerFromOpenApi(code, folderName)
			if (httpTriggers.length === 0) {
				sendUserToast('No paths defined in the OpenAPI spec. Cannot generate HTTP routes.', true)
			}
		} catch (error) {
			sendUserToast(error.message || 'An unexpected error occurred', true)
		} finally {
			isGeneratingHttpRoutes = false
		}
	}
</script>

<RouteEditor customSaveBehavior={callback} bind:this={routeEditor} />

<Drawer size="700px" bind:this={routesGenerator}>
	<DrawerContent
		title={'Generate HTTP routes from OpenAPI spec'}
		on:close={() => routesGenerator.closeDrawer()}
	>
		{#snippet actions()}
			<Button
				size="sm"
				loading={isCreating}
				disabled={httpTriggers.length === 0}
				startIcon={{ icon: Save }}
				on:click={saveHttpTrigger}
			>
				Save routes
			</Button>
		{/snippet}
		{@render config()}
	</DrawerContent>
</Drawer>

{#snippet config()}
	<div class="h-full">
		<Section label="HTTP routes generator" headless wrapperClass="h-full">
			<div class="flex flex-col gap-1 h-full">
				<Subsection>
					<div class="flex flex-col gap-1">
						<p class="text-xs text-primary">
							Pick a folder to bind the generated HTTP routes to.<Required required={true} />
						</p>
						<FolderPicker bind:folderName />
					</div>
				</Subsection>

				{#if !emptyStringTrimmed(folderName)}
					<Subsection>
						<ToggleButtonGroup
							on:selected={({ detail }) => {
								if (selected === 'OpenAPI') {
									openApiRawEditorValue = code
								} else if (selected === 'OpenAPI_File') {
									openApiFileEditorValue = code
								} else {
									openApiUrlEditorValue = code
								}

								const type = detail as Source

								if (type === 'OpenAPI') {
									code = openApiRawEditorValue
								} else if (type === 'OpenAPI_File') {
									code = openApiFileEditorValue
								} else {
									code = openApiUrlEditorValue
								}
								forceRerender = !forceRerender
							}}
							bind:selected
						>
							{#snippet children({ item })}
								<ToggleButton
									tooltip="Paste an OpenAPI JSON/YAML specification directly to generate HTTP routes."
									showTooltipIcon
									label="Paste OpenAPI spec"
									value="OpenAPI"
									{item}
								/>
								<ToggleButton
									tooltip="Upload an OpenAPI file in JSON/YAML format to generate HTTP routes."
									showTooltipIcon
									label="From OpenAPI file"
									value="OpenAPI_File"
									{item}
								/>
								<ToggleButton
									tooltip="Provide a publicly accessible URL to an OpenAPI specification in JSON/YAML format to generate HTTP routes."
									showTooltipIcon
									label="From OpenAPI URL"
									value="OpenAPI_URL"
									{item}
								/>
							{/snippet}
						</ToggleButtonGroup>

						<div class="flex flex-col gap-2 mt-2">
							{#if selected === 'OpenAPI'}
								<ToggleButtonGroup bind:selected={lang}>
									{#snippet children({ item })}
										<ToggleButton value="yaml" label="YAML" {item} />
										<ToggleButton value="json" label="JSON" {item} />
									{/snippet}
								</ToggleButtonGroup>
							{:else if selected === 'OpenAPI_File'}
								<FileInput
									accept={acceptedFileTypes.join(',')}
									multiple={false}
									convertTo={'text'}
									iconSize={24}
									returnFileNames={true}
									class="text-sm py-4"
									on:change={async ({ detail }) => {
										if (detail && detail.length > 0) {
											if (openApiFile.endsWith('.json')) {
												lang = 'json'
											} else {
												lang = 'yaml'
											}
											code = detail[0].data
											openApiFileEditorValue = code
											openApiFile = detail[0].name as string
											forceRerender = !forceRerender
											return
										}
										openApiFile = ''
									}}
								/>
							{:else if selected === 'OpenAPI_URL'}
								<div class="flex flex-row gap-1">
									<input type="text" placeholder="OpenAPI URL" bind:value={openApiUrl} />
									<Button
										color="light"
										spacingSize="sm"
										size="xs"
										loading={isFetchingOpenApiSpec}
										disabled={!isValidUrl}
										on:click={fetchOpenApiConfig}
									>
										Fetch
									</Button>
								</div>
							{/if}
							{#if selected === 'OpenAPI' || (selected === 'OpenAPI_File' && !emptyStringTrimmed(openApiFile)) || (selected === 'OpenAPI_URL' && !emptyStringTrimmed(openApiUrl))}
								{#key forceRerender}
									<SimpleEditor class="h-96" {lang} bind:code />
								{/key}
							{/if}
							<Button
								spacingSize="sm"
								size="xs"
								btnClasses="h-8"
								loading={isGeneratingHttpRoutes}
								on:click={generateHttpTrigger}
								disabled={code.length === 0}
								variant="default">Generate HTTP routes</Button
							>
						</div>
					</Subsection>

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
												httpTriggers[index] = newHttpTrigger as NewHttpTrigger
												httpTriggers = httpTriggers
											}
											openRouteEditor(httpTriggers[index].path, httpTriggers[index])
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
				{/if}
			</div>
		</Section>
	</div>
{/snippet}
