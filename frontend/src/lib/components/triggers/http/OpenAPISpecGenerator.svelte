<script lang="ts">
	import { Badge, DrawerContent } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import Required from '$lib/components/Required.svelte'
	import Section from '$lib/components/Section.svelte'
	import Subsection from '$lib/components/Subsection.svelte'
	import {
		HttpTriggerService,
		type OpenapiHttpRouteFilters,
		type OpenapiSpecFormat
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { base } from '$lib/base'
	import Label from '$lib/components/Label.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { Trash } from 'lucide-svelte'

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
	let openapiDocument = $state('')
	let httpRouteFilters: OpenapiHttpRouteFilters[] = $state([])
	let webhookFilters: string[] = $state([])
	let lang: OpenapiSpecFormat = $state('json')

	async function generateOpenapiSpec() {
		openapiDocument = await HttpTriggerService.generateOpenapiSpec({
			workspace: $workspaceStore!,
			requestBody: {
				openapi_spec_format: lang,
				info: undefined,
				url: `${window.location.origin}${base}/api/r`,
				http_route_filters: httpRouteFilters,
				webhook_filters: webhookFilters
			}
		})
	}

	export function openDrawer() {
		openAPIGenerator.openDrawer()
	}
</script>

<Drawer size="1400px" bind:this={openAPIGenerator}>
	<DrawerContent
		title={'Generate OpenAPI document'}
		on:close={() => openAPIGenerator.closeDrawer()}
	>
		<div class="flex flex-row h-full gap-2">
			<div class="h-full w-1/2">
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
						<Subsection label="Filters">
							<Subsection label="Http route filter" headless>
								<div class="flex flex-col gap-2">
									{#if httpRouteFilters.length > 0}
										<div class="flex flex-col gap-2">
											{#each httpRouteFilters as httpRouteFilter, i}
												{@const postgresqlRegex = `f/${httpRouteFilter.folder_regex}/${httpRouteFilter.path_regex}`}

												<div class="flex flex-row items-center gap-2">
													<div class="border w-full rounded-md py-2 px-4 gap-2">
														<Label label="Route path filter" class="w-full">
															<svelte:fragment slot="header">
																<Tooltip small>
																	<p>
																		Match routes using a simple pattern. For example, <code
																			>/user/*</code
																		>
																		will match any route that starts with <code>/user/</code>, such
																		as
																		<code>/user/123</code>
																		or <code>/user/:id</code>. Only the <code>*</code> wildcard is supported.
																	</p>
																</Tooltip>
															</svelte:fragment>

															<input
																type="text"
																placeholder="products"
																bind:value={httpRouteFilter.route_path_regex}
																onchange={() => {
																	if (httpRouteFilter.route_path_regex.length === 0) {
																		httpRouteFilter.route_path_regex = '*'
																	}
																}}
															/>
														</Label>

														<Label label="Path filter">
															<div class="flex flex-col gap-2">
																<div
																	class="flex flex-col sm:flex-row sm:items-center gap-2 sm:gap-4 pb-0 mb-1"
																>
																	<div>
																		<input
																			bind:value={httpRouteFilter.folder_regex}
																			onchange={() => {
																				if (httpRouteFilter.folder_regex.length === 0) {
																					httpRouteFilter.folder_regex = '*'
																				}
																			}}
																			type="text"
																			id="folder"
																			autocomplete="off"
																		/>
																	</div>
																	<span class="text-xl">/</span>
																	<label class="block grow w-full max-w-md">
																		<input
																			bind:value={httpRouteFilter.path_regex}
																			onchange={() => {
																				if (httpRouteFilter.path_regex?.length === 0) {
																					httpRouteFilter.path_regex = '*'
																				}
																			}}
																			type="text"
																			id="path"
																			autocomplete="off"
																		/>
																	</label>
																</div>

																<div class="flex flex-col w-full">
																	<div class="flex justify-start w-full">
																		<Badge
																			color="gray"
																			class="center-center !bg-surface-secondary !text-tertiary !w-[70px] !h-[24px] rounded-r-none border"
																		>
																			Full path
																		</Badge>
																		<input
																			type="text"
																			readonly
																			value={postgresqlRegex}
																			size={postgresqlRegex.length}
																			class="font-mono !text-xs max-w-[calc(100%-70px)] !w-auto !h-[24px] !py-0 !border-l-0 !rounded-l-none"
																		/>
																	</div>
																</div>
															</div>
														</Label>
													</div>
													<Button
														variant="border"
														color="light"
														size="xs"
														btnClasses="bg-surface-secondary hover:bg-red-500 hover:text-white p-2 rounded-full"
														aria-label="Clear"
														on:click={() => {
															httpRouteFilters = httpRouteFilters.filter((_, index) => index !== i)
														}}
													>
														<Trash size={14} />
													</Button>
												</div>
											{/each}
										</div>
									{/if}
									<Button
										spacingSize="sm"
										size="xs"
										color="light"
										variant="border"
										on:click={() => {
											httpRouteFilters.push({
												path_regex: '*',
												folder_regex: '*',
												route_path_regex: '*'
											})
										}}>Add HTTP routes filter</Button
									>
								</div>
							</Subsection>
						</Subsection>
						<Button
							disabled={httpRouteFilters.length === 0}
							spacingSize="sm"
							size="xs"
							btnClasses="mb-2"
							loading={isLoadingHttpTriggers}
							on:click={generateOpenapiSpec}
							color="light"
							variant="border">Generate OpenAPI document</Button
						>
					</div>
				</Section>
			</div>
			<div class="w-1/2 h-full flex flex-col gap-2">
				<ToggleButtonGroup bind:selected={lang} let:item>
					<ToggleButton value="json" label="json" {item} />
					<ToggleButton value="yaml" label="yaml" {item} />
				</ToggleButtonGroup>
				{#key openapiDocument}
					<SimpleEditor class="h-full" {lang} code={openapiDocument} />
				{/key}
			</div>
		</div>
	</DrawerContent>
</Drawer>
