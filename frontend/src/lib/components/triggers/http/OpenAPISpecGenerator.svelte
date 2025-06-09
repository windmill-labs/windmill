<script lang="ts">
	import { DrawerContent } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import Required from '$lib/components/Required.svelte'
	import Section from '$lib/components/Section.svelte'
	import Subsection from '$lib/components/Subsection.svelte'
	import { HttpTriggerService, type HttpTrigger } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'

	let openAPIGenerator: Drawer

	let title = $state('')
	let version = $state('')
	let description = $state('')
	let contactName = $state('')
	let contactEmail = $state('')
	let contactUrl = $state('')
	let licenseName = $state('')
	let licenseUrl = $state('')
	let isLoadingHttpTriggers = false

	let httpRoutes: HttpTrigger[] = $state([])

	async function loadHttpRoutes() {
		try {
			httpRoutes = await HttpTriggerService.listHttpTriggers({
				workspace: $workspaceStore!
			})
		} catch (error) {
			sendUserToast(error.body, true)
		} finally {
			isLoadingHttpTriggers = true
		}
	}

	export function openDrawer() {
		openAPIGenerator.openDrawer()
	}
</script>

<Drawer size="700px" bind:this={openAPIGenerator}>
	<DrawerContent
		title={'Generate OpenAPI spec from HTTP routes'}
		on:close={() => openAPIGenerator.closeDrawer()}
	>
		<Section label="Generate OpenAPI spec" headless>
			<div class="flex flex-col gap-2">
				<Subsection label="Info" collapsable>
					<label class="block pb-2">
						<div class="flex flex-col">
							<span class="text-primary font-semibold text-sm">Title <Required required /></span>
							<span class="text-xs text-gray-500">The name of your API. Required for OpenAPI.</span>
						</div>
						<input type="text" placeholder="API Title" bind:value={title} />
					</label>

					<label class="block pb-2">
						<div class="flex flex-col">
							<span class="text-primary font-semibold text-sm">Version <Required required /></span>
							<span class="text-xs text-gray-500"
								>API version. Use semantic versioning (e.g., 1.0.0).</span
							>
						</div>
						<input type="text" placeholder="1.0.0" bind:value={version} />
					</label>

					<label class="block pb-2">
						<div class="flex flex-col">
							<span class="text-primary font-semibold text-sm">Description</span>
							<span class="text-xs text-gray-500"
								>Optional longer description of what your API does.</span
							>
						</div>
						<textarea placeholder="Optional API-wide description" bind:value={description}
						></textarea>
					</label>

					<label class="block pb-2">
						<div class="flex flex-col">
							<span class="text-primary font-semibold text-sm">Contact Name</span>
							<span class="text-xs text-gray-500"
								>Person responsible for the API (e.g., maintainer or team).</span
							>
						</div>
						<input type="text" placeholder="Jane Doe" bind:value={contactName} />
					</label>

					<label class="block pb-2">
						<div class="flex flex-col">
							<span class="text-primary font-semibold text-sm">Contact Email</span>
							<span class="text-xs text-gray-500"
								>Email address for API-related questions or issues.</span
							>
						</div>
						<input type="email" placeholder="jane@example.com" bind:value={contactEmail} />
					</label>

					<label class="block pb-2">
						<div class="flex flex-col">
							<span class="text-primary font-semibold text-sm">Contact URL</span>
							<span class="text-xs text-gray-500"
								>Link to documentation, team page, or relevant site.</span
							>
						</div>
						<input type="url" placeholder="https://example.com" bind:value={contactUrl} />
					</label>

					<label class="block pb-2">
						<div class="flex flex-col">
							<span class="text-primary font-semibold text-sm">License Name</span>
							<span class="text-xs text-gray-500">Type of license (e.g., MIT, Apache 2.0).</span>
						</div>
						<input type="text" placeholder="MIT" bind:value={licenseName} />
					</label>

					<label class="block pb-2">
						<div class="flex flex-col">
							<span class="text-primary font-semibold text-sm">License URL</span>
							<span class="text-xs text-gray-500">Link to the license terms and conditions.</span>
						</div>
						<input
							type="url"
							placeholder="https://opensource.org/licenses/MIT"
							bind:value={licenseUrl}
						/>
					</label>
				</Subsection>
				<Subsection label="servers" collapsable></Subsection>
				<Subsection label="paths">
                    
                </Subsection>
			</div>
		</Section>
	</DrawerContent>
</Drawer>
