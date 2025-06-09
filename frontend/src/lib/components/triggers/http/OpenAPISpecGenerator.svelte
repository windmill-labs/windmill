<script lang="ts">
	import { DrawerContent } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import Required from '$lib/components/Required.svelte'
	import Section from '$lib/components/Section.svelte'
	import Subsection from '$lib/components/Subsection.svelte'
	import { HttpTriggerService, type HttpTrigger } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Eye, Notebook, Trash } from 'lucide-svelte'
	import RouteEditor from './RouteEditor.svelte'
	import { generateOpenAPIspec } from './utils'
	import type { OpenAPIV3_1 } from 'openapi-types'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'

	let openAPIGenerator: Drawer

	let title = $state('')
	let version = $state('')
	let description = $state('')
	let contactName = $state('')
	let contactEmail = $state('')
	let contactUrl = $state('')
	let licenseName = $state('')
	let licenseUrl = $state('')
	let isLoadingHttpTriggers = $state(false)
	let servers: OpenAPIV3_1.ServerObject[] = $state([])
	let codeViewer: Drawer | undefined = $state()
	let editor: SimpleEditor | undefined = $state()
	let pathStart = $state('')
	let regexPath = $state('')
	let folder = $state('')
	let generatedOpenAPIspec = $state('')
	let httpRoutes: HttpTrigger[] = $state([])
	let routeEditor: RouteEditor
	let callback: ((cfg?: Record<string, unknown>) => void) | undefined = $state()
	async function loadHttpRoutes() {
		try {
			isLoadingHttpTriggers = true
			httpRoutes = await HttpTriggerService.listHttpTriggers({
				pathStart,
				workspace: $workspaceStore!
			})
			console.log({ httpRoutes })
		} catch (error) {
			sendUserToast(error.body, true)
		} finally {
			isLoadingHttpTriggers = false
		}
	}

	function generateSpec() {
		const info: OpenAPIV3_1.InfoObject = {
			title,
			version,
			description,
			contact: {
				name: contactName,
				url: contactUrl,
				email: contactEmail
			},
			license: {
				name: licenseName,
				url: licenseUrl
			}
		}

		generatedOpenAPIspec = generateOpenAPIspec(info, servers, httpRoutes)
		codeViewer?.openDrawer()
	}

	export function openDrawer() {
		openAPIGenerator.openDrawer()
	}
</script>

<RouteEditor onUpdate={callback} bind:this={routeEditor} />

<Drawer bind:this={codeViewer} size="600px">
	<DrawerContent title="OpenAPI spec" on:close={codeViewer.closeDrawer}>
		<SimpleEditor
			on:focus
			on:blur
			bind:this={editor}
			on:change
			autoHeight
			lang="json"
			bind:code={generatedOpenAPIspec}
		/>
	</DrawerContent>
</Drawer>

<Drawer size="700px" bind:this={openAPIGenerator}>
	<DrawerContent
		title={'Generate OpenAPI document'}
		on:close={() => openAPIGenerator.closeDrawer()}
	>
		<svelte:fragment slot="actions">
			<Button
				disabled={httpRoutes.length === 0}
				startIcon={{ icon: Notebook }}
				on:click={generateSpec}>Generate</Button
			>
		</svelte:fragment>
		<Section label="Generate OpenAPI spec" wrapperClass="h-full" headless>
			<div class="flex flex-col gap-2">
				<Subsection label="Info" collapsable>
					<label class="block pb-2">
						<span class="text-xs text-gray-500"
							>The name of your API. Required for OpenAPI. <Required required={true} />
						</span>
						<input type="text" placeholder="API Title" bind:value={title} />
					</label>

					<label class="block pb-2">
						<span class="text-xs text-gray-500"
							>API version. Use semantic versioning (e.g., 1.0.0).</span
						>
						<input type="text" placeholder="1.0.0" bind:value={version} />
					</label>

					<label class="block pb-2">
						<span class="text-xs text-gray-500"
							>Optional longer description of what your API does.</span
						>
						<textarea placeholder="Optional API-wide description" bind:value={description}
						></textarea>
					</label>

					<label class="block pb-2">
						<span class="text-xs text-gray-500"
							>Person responsible for the API (e.g., maintainer or team).</span
						>
						<input type="text" placeholder="Jane Doe" bind:value={contactName} />
					</label>

					<label class="block pb-2">
						<span class="text-xs text-gray-500"
							>Email address for API-related questions or issues.</span
						>
						<input type="email" placeholder="jane@example.com" bind:value={contactEmail} />
					</label>

					<label class="block pb-2">
						<span class="text-xs text-gray-500"
							>Link to documentation, team page, or relevant site.</span
						>
						<input type="url" placeholder="https://example.com" bind:value={contactUrl} />
					</label>

					<label class="block pb-2">
						<span class="text-xs text-gray-500">Type of license (e.g., MIT, Apache 2.0).</span>
						<input type="text" placeholder="MIT" bind:value={licenseName} />
					</label>

					<label class="block pb-2">
						<span class="text-xs text-gray-500">Link to the license terms and conditions.</span>
						<input
							type="url"
							placeholder="https://opensource.org/licenses/MIT"
							bind:value={licenseUrl}
						/>
					</label>
				</Subsection>
				<Subsection label="servers" collapsable></Subsection>
				<Subsection label="paths" headless>
					<label class="block pb-2">
						<span class="text-xs text-gray-500"
							>Fetch routes with specific folder, if left empty all routes will be fetched.</span
						>
						<input type="text" placeholder="test" bind:value={folder} />
					</label>
					<label class="block pb-2">
						<span class="text-xs text-gray-500"
							>Fetch routes that start with a specific string in their path, if left empty all
							triggers will be loaded.</span
						>
						<input type="text" placeholder="products" bind:value={pathStart} />
					</label>
					<label class="block pb-2">
						<span class="text-xs text-gray-500"
							>Fetch routes with matching the specified regex, if left empty all trigger will be
							fetch.</span
						>
						<input type="text" placeholder="test" bind:value={regexPath} />
					</label>

					<Button
						spacingSize="sm"
						size="xs"
						btnClasses="mb-2"
						loading={isLoadingHttpTriggers}
						on:click={loadHttpRoutes}
						color="light"
						variant="border">Fetch HTTP routes</Button
					>

					<div class="flex flex-col gap-2 grow min-h-0 overflow-y-auto">
						{#each httpRoutes as httpRoute, i}
							<div
								class="w-full flex flex-row justify-between content-center gap-2 border py-2 px-4 rounded-md"
							>
								<p class="content-center">{httpRoute.path}</p>
								<div class="flex flex-row gap-2">
									<Button
										spacingSize="sm"
										size="xs"
										btnClasses="h-8"
										color="light"
										variant="border"
										startIcon={{ icon: Eye }}
										iconOnly
										on:click={() => {
											callback = (cfg?: Record<string, unknown>) => {
												console.log({ cfg })
											}
											routeEditor.openEdit(httpRoute.path, httpRoute.is_flow)
										}}
									/>
									<Button
										variant="border"
										color="light"
										size="xs"
										btnClasses="bg-surface-secondary hover:bg-red-500 hover:text-white p-2 rounded-full"
										aria-label="Clear"
										on:click={() => {
											if (httpRoutes) {
												httpRoutes = httpRoutes.filter((_, index) => index !== i)
											}
										}}
									>
										<Trash size={14} />
									</Button>
								</div>
							</div>
						{/each}
					</div>
				</Subsection>
			</div>
		</Section>
	</DrawerContent>
</Drawer>
