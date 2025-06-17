<script lang="ts">
	import { Badge, DrawerContent } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import Required from '$lib/components/Required.svelte'
	import Subsection from '$lib/components/Subsection.svelte'
	import {
		HttpTriggerService,
		type OpenapiHttpRouteFilters,
		type OpenapiSpecFormat,
		type OpenapiV3Info,
		type WebhookFilters
	} from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { base } from '$lib/base'
	import Label from '$lib/components/Label.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { ClipboardCopy, Download, Trash } from 'lucide-svelte'
	import Select from '$lib/components/Select.svelte'
	import { sendUserToast } from '$lib/toast'
	import {
		copyToClipboard,
		download,
		emptyString,
		emptyStringTrimmed,
		generateRandomString
	} from '$lib/utils'
	import YAML from 'yaml'
	import { tick } from 'svelte'
	import CopyableCodeBlock from '$lib/components/details/CopyableCodeBlock.svelte'
	import { bash } from 'svelte-highlight/languages'
	import CreateToken from '$lib/components/settings/CreateToken.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'

	type HttpRouteAndWebhook = WebhookFilters | OpenapiHttpRouteFilters

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
	let lang: OpenapiSpecFormat = $state('yaml')
	let editor: SimpleEditor | undefined = $state()
	let generateCurlCommandDrawer: Drawer | undefined = $state()
	let webhookAndHttpRouteFilter: HttpRouteAndWebhook[] = $state([])
	let disabled = $derived(webhookAndHttpRouteFilter.length === 0)
	let token = $state('')
	let obj: Record<string, unknown> = $state({})

	function isWebhookFilter(data: HttpRouteAndWebhook): data is WebhookFilters {
		return (data as WebhookFilters).runnable_kind !== undefined
	}

	function isHttpRouteFilter(data: HttpRouteAndWebhook): data is OpenapiHttpRouteFilters {
		return (data as OpenapiHttpRouteFilters).route_path_regex !== undefined
	}

	function getWebhookFilters() {
		return webhookAndHttpRouteFilter.filter(isWebhookFilter)
	}

	function getHttpRouteFilters() {
		return webhookAndHttpRouteFilter.filter(isHttpRouteFilter)
	}

	function buildInfo() {
		const isTitle = !emptyString(title)
		const isVersion = !emptyString(version)

		let info: OpenapiV3Info | undefined = undefined

		if (isTitle && !isVersion) {
			throw new Error('Please fill version input')
		} else if (!isTitle && isVersion) {
			throw new Error('Please fill title input')
		} else if (isTitle && isVersion) {
			let isContact =
				!emptyString(contactName) || !emptyString(contactUrl) || !emptyString(contactEmail)
			let isLicense = !emptyString(licenseName) || !emptyString(licenseUrl)
			info = {
				title,
				version,
				description,
				contact: isContact
					? {
							name: contactName,
							url: contactUrl,
							email: contactEmail
						}
					: undefined,
				license: isLicense
					? {
							name: licenseName,
							url: licenseUrl
						}
					: undefined
			}
		}
		return info
	}

	async function copyCommandToClipboard() {
		try {
			const info = buildInfo()
			obj = {
				openapi_spec_format: lang,
				info,
				url: `${window.location.origin}${base}`,
				http_route_filters: getHttpRouteFilters(),
				webhook_filters: getWebhookFilters()
			}
			generateCurlCommandDrawer?.openDrawer()
		} catch (error) {
			sendUserToast(error.message || error, true)
		}
	}

	async function generateOpenapiSpec(openapi_spec_format: OpenapiSpecFormat) {
		try {
			const info = buildInfo()
			isGeneratingOpenapiSpec = true
			openapiDocument = await HttpTriggerService.generateOpenapiSpec({
				workspace: $workspaceStore!,
				requestBody: {
					openapi_spec_format,
					info,
					url: `${window.location.origin}${base}`,
					http_route_filters: getHttpRouteFilters(),
					webhook_filters: getWebhookFilters()
				}
			})
		} catch (error) {
			openapiDocument = ''
			sendUserToast(error.body || error.message, true)
		} finally {
			editor?.setCode(openapiDocument, true)
			isGeneratingOpenapiSpec = false
		}
	}

	export function openDrawer() {
		openAPIGenerator.openDrawer()
	}
</script>

<Drawer
	size="700px"
	bind:this={generateCurlCommandDrawer}
	on:close={() => {
		token = ''
	}}
>
	<DrawerContent on:close={generateCurlCommandDrawer.closeDrawer} title={''}>
		<div class="flex flex-col gap-2">
			<div class="flex flex-col gap-3">
				<Alert title="Full Access Warning" type="warning">
					Any token generated using the form below will grant unrestricted access to the Windmill
					API. Only share it with trusted parties.
				</Alert>
				<CreateToken
					on:tokenCreated={(e) => {
						token = e.detail
					}}
					newTokenLabel={`openapi-${$userStore?.username ?? 'superadmin'}-${generateRandomString(4)}`}
				/>
			</div>

			<Label label="Token">
				<input
					bind:value={token}
					placeholder="paste a windmill token to alter the example below"
					class="!text-xs !font-normal"
				/>
			</Label>

			<Label label="Example cURL">
				<svelte:fragment slot="header">
					<Tooltip>
						Use this cURL command to call the OpenAPI generation endpoint.

						<br /><br />

						You can either:
						<ul class="list-disc pl-5 mt-1 text-sm leading-snug">
							<li
								>Paste an existing Windmill token into the <code>token variable</code> placeholder</li
							>
							<li>Or generate a new token</li>
						</ul>

						<br />
					</Tooltip>
				</svelte:fragment>
				<CopyableCodeBlock
					code={`token=${emptyString(token) ? '' : token}; \\
curl -X POST "${window.location.origin}${base}/api/w/${$workspaceStore!}/http_triggers/openapi/generate" \\
-H "Authorization: Bearer $token" \\
-H "Content-Type: application/json" \\
-d '${JSON.stringify(obj)}'`}
					language={bash}
				/>
			</Label>
		</div>
	</DrawerContent>
</Drawer>

<Drawer size="1400px" bind:this={openAPIGenerator}>
	<DrawerContent
		title={'Generate OpenAPI document'}
		on:close={() => openAPIGenerator.closeDrawer()}
	>
		<div class="flex flex-row h-full gap-2">
			<div class="h-full w-1/2 flex flex-col justify-between border rounded-md">
				<div class="flex flex-col gap-2 overflow-auto pl-2 pt-2">
					<Subsection label="Info" collapsable>
						<label class="block pb-2">
							<span class="text-xs"
								>The name of your API. Required for OpenAPI. <Required required={true} /></span
							>
							<input type="text" placeholder="API Title" bind:value={title} />
						</label>
						<label class="block pb-2">
							<span class="text-xs"
								>API version. Use semantic versioning (e.g., 1.0.0). <Required
									required={true}
								/></span
							>
							<input type="text" placeholder="1.0.0" bind:value={version} />
						</label>
						<label class="block pb-2">
							<span class="text-xs">Optional longer description of what your API does.</span>
							<textarea placeholder="Optional API-wide description" bind:value={description}
							></textarea>
						</label>
						<label class="block pb-2">
							<span class="text-xs">Person responsible for the API (e.g., maintainer or team).</span
							>
							<input type="text" placeholder="Jane Doe" bind:value={contactName} />
						</label>
						<label class="block pb-2">
							<span class="text-xs">Email address for API-related questions or issues.</span>
							<input type="email" placeholder="jane@example.com" bind:value={contactEmail} />
						</label>
						<label class="block pb-2">
							<span class="text-xs">Link to documentation, team page, or relevant site.</span>
							<input type="url" placeholder="https://example.com" bind:value={contactUrl} />
						</label>
						<label class="block pb-2">
							<span class="text-xs">Type of license (e.g., MIT, Apache 2.0).</span>
							<input type="text" placeholder="MIT" bind:value={licenseName} />
						</label>
						<label class="block pb-2">
							<span class="text-xs">Link to the license terms and conditions.</span>
							<input
								type="url"
								placeholder="https://opensource.org/licenses/MIT"
								bind:value={licenseUrl}
							/>
						</label>
					</Subsection>

					{#if webhookAndHttpRouteFilter.length > 0}
						{#each webhookAndHttpRouteFilter as filter, i}
							{@const isHttpRouteFIlter = isHttpRouteFilter(filter)}
							<div class="flex flex-row items-center py-2 px-4 gap-2">
								<div class="border w-full rounded-md p-4 gap-2">
									<Label
										label={`${isHttpRouteFIlter ? 'HTTP route filter' : 'Webhook filter'}`}
										headerClass="font-bold"
									>
										<div class="mt-2">
											{#if isHttpRouteFIlter}
												{@const httpRouteFilter = filter}
												{@const postgresqlRegex = `f/${httpRouteFilter.folder_regex}/${httpRouteFilter.path_regex}`}
												<div class="flex flex-col gap-2">
													<Label label="Route path regex" class="w-full">
														<input
															type="text"
															placeholder="products"
															class="mt-1"
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
																class="flex flex-col sm:flex-row sm:items-center gap-2 sm:gap-4 pb-0 mt-1"
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
											{:else}
												{@const webhookFilters = filter}
												{@const postgresqlRegex = `${webhookFilters.user_or_folder_regex}/${webhookFilters.user_or_folder_regex_value}/${webhookFilters.path}`}

												<Label label="Filter" headless>
													<div class="flex flex-col gap-2">
														<ToggleButtonGroup bind:selected={webhookFilters.runnable_kind}>
															{#snippet children({ item })}
																<ToggleButton value="script" label="script" {item} />
																<ToggleButton value="flow" label="flow" {item} />
															{/snippet}
														</ToggleButtonGroup>

														<Label label="Path regex">
															<div class="flex flex-col gap-2">
																<div
																	class="flex flex-col sm:flex-row sm:items-center gap-2 sm:gap-4 pb-0 mt-1"
																>
																	<Select
																		clearable={false}
																		class="grow shrink"
																		bind:value={webhookFilters.user_or_folder_regex}
																		items={['*', 'u', 'f'].map((value) => ({ value }))}
																	/>
																	<span class="text-xl">/</span>
																	<div>
																		<input
																			bind:value={webhookFilters.user_or_folder_regex_value}
																			onchange={() => {
																				if (
																					webhookFilters.user_or_folder_regex_value.length === 0
																				) {
																					webhookFilters.user_or_folder_regex_value = '*'
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
																			bind:value={webhookFilters.path}
																			onchange={() => {
																				if (webhookFilters.path.length === 0) {
																					webhookFilters.path = '*'
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
											{/if}
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
										webhookAndHttpRouteFilter = webhookAndHttpRouteFilter.filter(
											(_, index) => index !== i
										)
									}}
								>
									<Trash size={14} />
								</Button>
							</div>
						{/each}
					{/if}
				</div>

				<div class="flex flex-col border-t w-full p-4 gap-2">
					<div class="flex flex-row gap-2 w-full">
						<div class="w-1/2">
							<Button
								spacingSize="sm"
								color="light"
								btnClasses="h-10"
								size="xs"
								variant="border"
								on:click={() => {
									webhookAndHttpRouteFilter.push({
										path: '*',
										user_or_folder_regex: '*',
										user_or_folder_regex_value: '*',
										runnable_kind: 'script'
									})
								}}
							>
								Add webhook filters
								<Tooltip>
									<span class="text-sm leading-snug block">
										Add a new webhook filter to include specific webhooks in the generated OpenAPI
										spec.
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
												Defining a <strong>path regex</strong>: matches the internal Windmill path
												of a script or flow.
												<br />
												Example: <code>f/notify/*</code> will match any webhook whose script or flow
												path starts with <code>f/notify/</code>, such as
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
						<div class="w-1/2">
							<Button
								size="xs"
								btnClasses="h-10"
								color="light"
								variant="border"
								on:click={() => {
									webhookAndHttpRouteFilter.push({
										path_regex: '*',
										folder_regex: '*',
										route_path_regex: '*'
									})
								}}
							>
								Add HTTP routes filters
								<Tooltip>
									<span class="text-sm leading-snug block">
										Add a new HTTP route filter to include specific routes in the generated OpenAPI
										spec.
									</span>

									<span class="text-sm leading-snug block mt-2"> You can define: </span>

									<ul class="list-disc pl-5 mt-1 text-sm leading-snug">
										<li>
											<span>
												<strong>route path regex</strong>: Matches the route path of the HTTP route.
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
					<Button
						{disabled}
						color="light"
						variant="border"
						spacingSize="sm"
						loading={isGeneratingOpenapiSpec}
						on:click={async () => {
							await generateOpenapiSpec(lang)
						}}
					>
						Generate OpenAPI document
					</Button>
					<Button
						{disabled}
						spacingSize="sm"
						color="light"
						size="xs"
						variant="border"
						on:click={copyCommandToClipboard}
						startIcon={{ icon: ClipboardCopy }}
					>
						Copy cURL command
					</Button>
				</div>
			</div>

			<div class="w-1/2 h-full flex flex-col gap-2">
				<div class="flex flex-row justify-between">
					<ToggleButtonGroup
						bind:selected={lang}
						on:selected={async ({ detail }) => {
							if (!emptyStringTrimmed(openapiDocument)) {
								try {
									if (detail === 'yaml') {
										const obj = JSON.parse(openapiDocument)
										openapiDocument = YAML.stringify(obj)
									} else {
										const obj = YAML.parse(openapiDocument)
										openapiDocument = JSON.stringify(obj, null, 2)
									}
									await tick()
									editor?.setCode(openapiDocument, true)
									await tick()
								} catch (error) {
									const message = error instanceof Error ? error.message : JSON.stringify(error)
									sendUserToast(`Conversion failed: ${message}`, true)
								}
							}
						}}
					>
						{#snippet children({ item })}
							<ToggleButton value="yaml" label="yaml" {item} />
							<ToggleButton value="json" label="json" {item} />
						{/snippet}
					</ToggleButtonGroup>
					<div class="flex flex-row gap-2">
						<Button
							disabled={emptyStringTrimmed(openapiDocument)}
							spacingSize="sm"
							color="light"
							variant="border"
							btnClasses="mb-2"
							on:click={async () => {
								await copyToClipboard(openapiDocument)
							}}
							startIcon={{ icon: ClipboardCopy }}
							iconOnly
						/>

						<Button
							disabled={emptyStringTrimmed(openapiDocument)}
							spacingSize="sm"
							color="light"
							variant="border"
							btnClasses="mb-2"
							on:click={() => {
								download(
									`openapi-3.1.${lang}`,
									openapiDocument,
									lang === 'yaml' ? 'text/x-yaml' : 'application/json'
								)
							}}
							startIcon={{ icon: Download }}
							iconOnly
						/>
					</div>
				</div>
				<SimpleEditor bind:this={editor} class="h-full" {lang} bind:code={openapiDocument} />
			</div>
		</div>
	</DrawerContent>
</Drawer>
