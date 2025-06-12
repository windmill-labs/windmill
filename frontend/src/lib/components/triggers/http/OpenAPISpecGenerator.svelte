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
		type OpenapiSpecFormat,
		type OpenapiV3Info,
		type WebhookFilters
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { base } from '$lib/base'
	import Label from '$lib/components/Label.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { Trash } from 'lucide-svelte'
	import Select from '$lib/components/Select.svelte'
	import { sendUserToast } from '$lib/toast'
	import { emptyString } from '$lib/utils'
	let openAPIGenerator: Drawer

	let title = $state('')
	let version = $state('')
	let description = $state('')
	let contactName = $state('')
	let contactEmail = $state('')
	let contactUrl = $state('')
	let licenseName = $state('')
	let licenseUrl = $state('')
	let isGeneratingOpenapiSpec = $state(false)
	let openapiDocument = $state('')
	let httpRouteFilters: OpenapiHttpRouteFilters[] = $state([])
	let webhookFilters: WebhookFilters[] = $state([])
	let lang: OpenapiSpecFormat = $state('yaml')
	let editor: SimpleEditor | undefined = $state()

	async function generateOpenapiSpec(openapi_spec_format: OpenapiSpecFormat) {
		try {
			const isTitle = !emptyString(title)
			const isVersion = !emptyString(version)

			let info: OpenapiV3Info | undefined = undefined

			if (isTitle && !isVersion) {
				sendUserToast('Please fill version input', true)
				return
			} else if (!isTitle && isVersion) {
				sendUserToast('Please fill title input', true)
				return
			} else if (isTitle && isVersion) {
				info = {
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
			}

			isGeneratingOpenapiSpec = true
			openapiDocument = await HttpTriggerService.generateOpenapiSpec({
				workspace: $workspaceStore!,
				requestBody: {
					openapi_spec_format,
					info,
					url: `${window.location.origin}${base}`,
					http_route_filters: httpRouteFilters,
					webhook_filters: webhookFilters
				}
			})
		} catch (error) {
			openapiDocument = ''
			sendUserToast(error.body, true)
		} finally {
			editor?.setCode(openapiDocument, true)
			isGeneratingOpenapiSpec = false
		}
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
		<svelte:fragment slot="actions">
			<Button
				disabled={httpRouteFilters.length === 0 && webhookFilters.length === 0}
				spacingSize="sm"
				btnClasses="mb-2"
				loading={isGeneratingOpenapiSpec}
				on:click={async () => {
					await generateOpenapiSpec(lang)
				}}
			>
				Generate OpenAPI document
			</Button>
		</svelte:fragment>

		<div class="flex flex-row h-full gap-2">
			<div class="h-full w-1/2">
				<Section label="Generate OpenAPI spec" wrapperClass="h-full" headless>
					<div class="flex flex-col gap-2 h-full">
						<Subsection label="Info" collapsable>
							<label class="block pb-2">
								<span class="text-xs text-gray-500"
									>The name of your API. Required for OpenAPI. <Required required={true} /></span
								>
								<input type="text" placeholder="API Title" bind:value={title} />
							</label>
							<label class="block pb-2">
								<span class="text-xs text-gray-500"
									>API version. Use semantic versioning (e.g., 1.0.0). <Required
										required={true}
									/></span
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

						<div class="flex flex-col gap-4 h-full">
							<div class="h-1/2 pr-2 flex flex-col gap-2">
								<div class="overflow-y-auto flex flex-col gap-2">
									{#if httpRouteFilters.length > 0}
										{#each httpRouteFilters as httpRouteFilter, i}
											{@const postgresqlRegex = `f/${httpRouteFilter.folder_regex}/${httpRouteFilter.path_regex}`}
											<div class="flex flex-row items-center gap-2">
												<div class="border w-full rounded-md py-2 px-4 gap-2">
													<Label label="Route path regex" class="w-full">
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

													<Label label="Path regex">
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
									{/if}
								</div>
								<div class="pr-10">
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
										}}
									>
										Add HTTP routes filters
										<Tooltip>
											<span class="text-sm leading-snug block">
												Add a new HTTP route filter to include specific routes in the generated
												OpenAPI spec.
											</span>

											<span class="text-sm leading-snug block mt-2"> You can define: </span>

											<ul class="list-disc pl-5 mt-1 text-sm leading-snug">
												<li>
													<span>
														<strong>route path regex</strong>: Matches the route path of the HTTP
														route.
														<br />
														Example: <code>/user/*</code> will match routes like
														<code>/user/123</code>
														or <code>/user/:id</code>.
													</span>
												</li>
												<li>
													<span>
														<strong>path regex</strong>: Matches the path of the route.
														<br />
														Example: <code>f/test/*</code> matches any route under the
														<code>f/test/</code> folder.
													</span>
												</li>
											</ul>

											<span class="text-sm leading-snug block mt-2">
												Only the wildcard <code>*</code> is supported in both regex fields.
											</span>
										</Tooltip>
									</Button>
								</div>
							</div>

							<div class="h-1/2 pr-2 flex flex-col gap-2">
								<div class="overflow-y-auto flex flex-col gap-2">
									{#if webhookFilters.length > 0}
										{#each webhookFilters as _, i}
											{@const postgresqlRegex = `${webhookFilters[i].user_or_folder_regex}/${webhookFilters[i].user_or_folder_regex_value}/${webhookFilters[i].path}`}
											<div class="flex flex-row items-center gap-2">
												<div class="border w-full rounded-md py-2 px-4 gap-2">
													<Label label="Filter" headless>
														<div class="flex flex-col gap-2">
															<ToggleButtonGroup
																bind:selected={webhookFilters[i].runnable_kind}
																let:item
															>
																<ToggleButton value="script" label="script" {item} />
																<ToggleButton value="flow" label="flow" {item} />
															</ToggleButtonGroup>

															<Label label="Path regex">
																<div class="flex flex-col gap-2">
																	<div
																		class="flex flex-col sm:flex-row sm:items-center gap-2 sm:gap-4 pb-0 mb-1"
																	>
																		<Select
																			clearable={false}
																			class="grow shrink"
																			bind:value={webhookFilters[i].user_or_folder_regex}
																			items={['*', 'u', 'f'].map((value) => ({ value }))}
																		/>
																		<span class="text-xl">/</span>
																		<div>
																			<input
																				bind:value={webhookFilters[i].user_or_folder_regex_value}
																				onchange={() => {
																					if (
																						webhookFilters[i].user_or_folder_regex_value.length ===
																						0
																					) {
																						webhookFilters[i].user_or_folder_regex_value = '*'
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
																				bind:value={webhookFilters[i].path}
																				onchange={() => {
																					if (webhookFilters[i].path.length === 0) {
																						webhookFilters[i].path = '*'
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
													</Label>
												</div>
												<Button
													variant="border"
													color="light"
													size="xs"
													btnClasses="bg-surface-secondary hover:bg-red-500 hover:text-white p-2 rounded-full"
													aria-label="Clear"
													on:click={() => {
														webhookFilters = webhookFilters.filter((_, index) => index !== i)
													}}
												>
													<Trash size={14} />
												</Button>
											</div>
										{/each}
									{/if}
								</div>
								<div class="pr-10">
									<Button
										spacingSize="sm"
										size="xs"
										color="light"
										variant="border"
										on:click={() => {
											webhookFilters.push({
												path: '*',
												user_or_folder_regex: '*',
												user_or_folder_regex_value: '*',
												runnable_kind: 'script'
											})
										}}
									>
										Add webhook filter
										<Tooltip>
											<span class="text-sm leading-snug block">
												Add a new webhook filter to include specific webhooks in the generated
												OpenAPI spec.
											</span>

											<span class="text-sm leading-snug block mt-2"> This filter works by: </span>

											<ul class="list-disc pl-5 mt-1 text-sm leading-snug">
												<li>
													<span>
														Selecting a <strong>runnable kind</strong>: either <code>script</code>
														or <code>flow</code>.
													</span>
												</li>
												<li>
													<span>
														Defining a <strong>path regex</strong>: matches the internal Windmill
														path of a script or flow.
														<br />
														Example: <code>f/notify/*</code> will match any webhook whose script or
														flow path starts with <code>f/notify/</code>, such as
														<code>f/notify/foo</code>
														or <code>f/notify/bar</code>.
													</span>
												</li>
											</ul>

											<span class="text-sm leading-snug block mt-2">
												Only the wildcard <code>*</code> is supported for matching.
											</span>
										</Tooltip>
									</Button>
								</div>
							</div>
						</div>
					</div>
				</Section>
			</div>

			<div class="w-1/2 h-full flex flex-col gap-2">
				<ToggleButtonGroup
					bind:selected={lang}
					on:selected={async ({ detail }) => {
						if (httpRouteFilters.length > 0 || webhookFilters.length > 0) {
							await generateOpenapiSpec(detail)
						}
					}}
					let:item
				>
					<ToggleButton value="yaml" label="yaml" {item} />
					<ToggleButton value="json" label="json" {item} />
				</ToggleButtonGroup>
				<SimpleEditor bind:this={editor} class="h-full" {lang} code={openapiDocument} />
			</div>
		</div>
	</DrawerContent>
</Drawer>
