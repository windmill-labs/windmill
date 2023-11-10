<script lang="ts">
	import {
		DraftService,
		NewScript,
		Script,
		ScriptService,
		WorkerService,
		type NewScriptWithDraft
	} from '$lib/gen'
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { inferArgs } from '$lib/infer'
	import { initialCode } from '$lib/script_helpers'
	import { enterpriseLicense, userStore, workerTags, workspaceStore } from '$lib/stores'
	import {
		cleanValueProperties,
		emptySchema,
		encodeState,
		getModifierKey,
		orderedJsonStringify
	} from '$lib/utils'
	import Path from './Path.svelte'
	import ScriptEditor from './ScriptEditor.svelte'
	import { Alert, Badge, Button, Drawer, Kbd, SecondsInput, Tab, TabContent, Tabs } from './common'
	import LanguageIcon from './common/languageIcons/LanguageIcon.svelte'
	import type { SupportedLanguage } from '$lib/common'
	import Tooltip from './Tooltip.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ErrorHandlerToggleButton from '$lib/components/details/ErrorHandlerToggleButton.svelte'
	import {
		Bug,
		CheckCircle,
		Code,
		DiffIcon,
		ExternalLink,
		Loader2,
		Pen,
		Plus,
		Rocket,
		Save,
		Settings,
		X
	} from 'lucide-svelte'
	import autosize from 'svelte-autosize'
	import type Editor from './Editor.svelte'
	import { SCRIPT_SHOW_BASH, SCRIPT_SHOW_GO } from '$lib/consts'
	import UnsavedConfirmationModal from './common/confirmationModal/UnsavedConfirmationModal.svelte'
	import { sendUserToast } from '$lib/toast'
	import { isCloudHosted } from '$lib/cloud'
	import Awareness from './Awareness.svelte'
	import { fade } from 'svelte/transition'
	import Popover from './Popover.svelte'
	import Toggle from './Toggle.svelte'
	import ScriptSchema from './ScriptSchema.svelte'
	import Section from './Section.svelte'
	import Label from './Label.svelte'
	import type DiffDrawer from './DiffDrawer.svelte'
	import { cloneDeep } from 'lodash'

	export let script: NewScript
	export let initialPath: string = ''
	export let template: 'pgsql' | 'mysql' | 'script' | 'docker' | 'powershell' = 'script'
	export let initialArgs: Record<string, any> = {}
	export let lockedLanguage = false
	export let showMeta: boolean = false
	export let diffDrawer: DiffDrawer | undefined = undefined
	export let savedScript: NewScriptWithDraft | undefined = undefined

	let metadataOpen =
		showMeta ||
		(initialPath == '' &&
			$page.url.searchParams.get('state') == undefined &&
			$page.url.searchParams.get('collab') == undefined)

	let editor: Editor | undefined = undefined
	let scriptEditor: ScriptEditor | undefined = undefined

	const enterpriseLangs = ['bigquery', 'snowflake', 'mssql']

	loadWorkerGroups()

	async function loadWorkerGroups() {
		if (!$workerTags) {
			$workerTags = await WorkerService.getCustomTags()
		}
	}

	export function setCode(code: string): void {
		editor?.setCode(code)
	}

	const langs: [string, SupportedLanguage][] = [
		['TypeScript (Deno)', Script.language.DENO],
		['Python', Script.language.PYTHON3],
		['TypeScript (Bun)', Script.language.BUN]
	]
	if (SCRIPT_SHOW_BASH) {
		langs.push(['Bash', Script.language.BASH])
	}
	if (SCRIPT_SHOW_GO) {
		langs.push(['Go', Script.language.GO])
	}
	langs.push(['REST', Script.language.NATIVETS])
	langs.push(['PostgreSQL', Script.language.POSTGRESQL])
	langs.push(['MySQL', Script.language.MYSQL])
	langs.push(['BigQuery', Script.language.BIGQUERY])
	langs.push(['Snowflake', Script.language.SNOWFLAKE])
	langs.push(['MS SQL Server', Script.language.MSSQL])
	langs.push(['GraphQL', Script.language.GRAPHQL])
	langs.push(['PowerShell', Script.language.POWERSHELL])

	const scriptKindOptions: {
		value: Script.kind
		title: string
		Icon: any
		desc?: string
		documentationLink?: string
	}[] = [
		{
			value: Script.kind.SCRIPT,
			title: 'Action',
			Icon: Code
		},
		{
			value: Script.kind.TRIGGER,
			title: 'Trigger',
			desc: 'First module of flows to trigger them based on external changes. These kind of scripts are usually running on a schedule to periodically look for changes.',
			documentationLink: 'https://www.windmill.dev/docs/flows/flow_trigger',
			Icon: Rocket
		},
		{
			value: Script.kind.APPROVAL,
			title: 'Approval',
			desc: 'Send notifications externally to ask for approval to continue a flow.',
			documentationLink: 'https://www.windmill.dev/docs/flows/flow_approval',
			Icon: CheckCircle
		},
		{
			value: Script.kind.FAILURE,
			title: 'Error Handler',
			desc: 'Handle errors in flows after all retry attempts have been exhausted.',
			documentationLink: 'https://www.windmill.dev/docs/flows/flow_error_handler',
			Icon: Bug
		}
	]

	let pathError = ''
	let loadingSave = false
	let loadingDraft = false

	$: {
		;['collab', 'path'].forEach((x) => {
			if ($page.url.searchParams.get(x)) {
				$page.url.searchParams.delete(x)
			}
		})
	}

	$: window.history.replaceState(null, '', '#' + encodeState(script))

	if (script.content == '') {
		initContent(script.language, script.kind, template)
	}

	function initContent(
		language: SupportedLanguage,
		kind: Script.kind | undefined,
		template: 'pgsql' | 'mysql' | 'script' | 'docker' | 'powershell'
	) {
		scriptEditor?.disableCollaboration()
		script.content = initialCode(language, kind, template)
		scriptEditor?.inferSchema(script.content, language)
		if (script.content != editor?.getCode()) {
			setCode(script.content)
		}
	}

	async function editScript(): Promise<void> {
		loadingSave = true
		try {
			localStorage.removeItem(script.path)

			script.schema = script.schema ?? emptySchema()
			try {
				await inferArgs(script.language, script.content, script.schema as any)
			} catch (error) {
				sendUserToast(
					`The main signature was not parsable. This script is considered to be without main function`
				)
			}

			const newHash = await ScriptService.createScript({
				workspace: $workspaceStore!,
				requestBody: {
					path: script.path,
					summary: script.summary,
					description: script.description ?? '',
					content: script.content,
					parent_hash: script.parent_hash,
					schema: script.schema,
					is_template: script.is_template,
					language: script.language,
					kind: script.kind,
					tag: script.tag,
					envs: script.envs,
					dedicated_worker: script.dedicated_worker,
					concurrent_limit: script.concurrent_limit,
					concurrency_time_window_s: script.concurrency_time_window_s,
					cache_ttl: script.cache_ttl,
					ws_error_handler_muted: script.ws_error_handler_muted,
					priority: script.priority
				}
			})
			savedScript = cloneDeep(script) as NewScriptWithDraft
			history.replaceState(history.state, '', `/scripts/edit/${script.path}`)
			goto(`/scripts/get/${newHash}?workspace=${$workspaceStore}`)
		} catch (error) {
			sendUserToast(`Error while saving the script: ${error.body || error.message}`, true)
		}
		loadingSave = false
	}

	async function saveDraft(forceSave = false): Promise<void> {
		if (initialPath != '' && !savedScript) {
			return
		}
		if (savedScript) {
			const draftOrDeployed = cleanValueProperties(savedScript.draft || savedScript)
			const current = cleanValueProperties(script)
			if (!forceSave && orderedJsonStringify(draftOrDeployed) === orderedJsonStringify(current)) {
				sendUserToast('No changes detected, ignoring', false, [
					{
						label: 'Save anyway',
						callback: () => {
							saveDraft(true)
						}
					}
				])
				return
			}
		}

		loadingDraft = true
		try {
			localStorage.removeItem(script.path)

			script.schema = script.schema ?? emptySchema()
			try {
				await inferArgs(script.language, script.content, script.schema as any)
			} catch (error) {
				sendUserToast(
					`The main signature was not parsable. This script is considered to be without main function`
				)
			}

			if (initialPath == '') {
				await ScriptService.createScript({
					workspace: $workspaceStore!,
					requestBody: {
						path: script.path,
						summary: script.summary,
						description: script.description ?? '',
						content: script.content,
						schema: script.schema,
						is_template: script.is_template,
						language: script.language,
						kind: script.kind,
						tag: script.tag,
						draft_only: true,
						envs: script.envs,
						concurrent_limit: script.concurrent_limit,
						concurrency_time_window_s: script.concurrency_time_window_s,
						cache_ttl: script.cache_ttl,
						ws_error_handler_muted: script.ws_error_handler_muted,
						priority: script.priority
					}
				})
			}
			await DraftService.createDraft({
				workspace: $workspaceStore!,
				requestBody: {
					path: initialPath == '' ? script.path : initialPath,
					typ: 'script',
					value: script
				}
			})

			savedScript = {
				...(initialPath == '' ? { ...cloneDeep(script), draft_only: true } : savedScript),
				draft: cloneDeep(script)
			} as NewScriptWithDraft

			if (initialPath == '') {
				goto(`/scripts/edit/${script.path}`)
			}
			sendUserToast('Saved as draft')
		} catch (error) {
			sendUserToast(
				`Error while saving the script as a draft: ${error.body || error.message}`,
				true
			)
		}
		loadingDraft = false
	}

	function computeDropdownItems(initialPath: string) {
		let dropdownItems: { label: string; onClick: () => void }[] =
			initialPath != ''
				? [
						{
							label: 'Fork',
							onClick: () => {
								window.open(`/scripts/add?template=${initialPath}`)
							}
						},
						...(!script.draft_only
							? [
									{
										label: 'Exit & See details',
										onClick: () => {
											goto(`/scripts/get/${initialPath}?workspace=${$workspaceStore}`)
										}
									}
							  ]
							: [])
				  ]
				: []

		return dropdownItems.length > 0 ? dropdownItems : undefined
	}

	function onKeyDown(event: KeyboardEvent) {
		switch (event.key) {
			case 's':
				if (event.ctrlKey || event.metaKey) {
					saveDraft()
					event.preventDefault()
				}
				break
		}
	}

	let path: Path | undefined = undefined
	let dirtyPath = false

	let selectedTab: 'metadata' | 'runtime' | 'ui' = 'metadata'
</script>

<svelte:window on:keydown={onKeyDown} />
<UnsavedConfirmationModal {diffDrawer} savedValue={savedScript} modifiedValue={script} />

{#if !$userStore?.operator}
	<Drawer placement="right" bind:open={metadataOpen} size="800px">
		<DrawerContent noPadding title="Settings" on:close={() => (metadataOpen = false)}>
			<!-- svelte-ignore a11y-autofocus -->
			<Tabs bind:selected={selectedTab}>
				<Tab value="metadata">Metadata</Tab>
				<Tab value="runtime">Runtime</Tab>
				<Tab value="ui">
					Generated UI
					<Tooltip
						documentationLink="https://www.windmill.dev/docs/core_concepts/json_schema_and_parsing"
					>
						The arguments are synced with the main signature but you may refine the parts that
						cannot be inferred from the type directly.
					</Tooltip>
				</Tab>
				<svelte:fragment slot="content">
					<div class="p-4">
						<TabContent value="metadata">
							<div class="flex flex-col gap-8">
								<Section label="Metadata">
									<svelte:fragment slot="action">
										<div class="flex flex-row items-center gap-2">
											<ErrorHandlerToggleButton
												kind="script"
												scriptOrFlowPath={script.path}
												bind:errorHandlerMuted={script.ws_error_handler_muted}
												iconOnly={false}
											/>
										</div>
									</svelte:fragment>
									<div class="flex flex-col gap-4">
										<Label label="Summary">
											<input
												type="text"
												autofocus
												bind:value={script.summary}
												placeholder="Short summary to be displayed when listed"
												on:keyup={() => {
													if (initialPath == '' && script.summary?.length > 0 && !dirtyPath) {
														path?.setName(
															script.summary
																.toLowerCase()
																.replace(/[^a-z0-9_]/g, '_')
																.replace(/-+/g, '_')
																.replace(/^-|-$/g, '')
														)
													}
												}}
											/>
										</Label>
										<Label label="Path">
											<Path
												bind:this={path}
												bind:error={pathError}
												bind:path={script.path}
												bind:dirty={dirtyPath}
												{initialPath}
												autofocus={false}
												namePlaceholder="script"
												kind="script"
											/>
										</Label>
										<Label label="Description">
											<textarea
												use:autosize
												bind:value={script.description}
												placeholder="Description displayed in the details page"
												class="text-sm"
											/>
										</Label>
									</div>
								</Section>

								<Section label="Language">
									{#if lockedLanguage}
										<div class="text-sm text-tertiary italic mb-2">
											As a forked script, the language '{script.language}' cannot be modified.
										</div>
									{/if}
									<div class=" grid grid-cols-3 gap-2">
										{#each langs as [label, lang]}
											{@const isPicked = script.language == lang && template == 'script'}
											<Popover
												disablePopup={!enterpriseLangs.includes(lang) || !!$enterpriseLicense}
											>
												<Button
													size="sm"
													variant="border"
													color={isPicked ? 'blue' : 'light'}
													btnClasses={isPicked
														? '!border-2 !bg-blue-50/75 dark:!bg-frost-900/75'
														: 'm-[1px]'}
													on:click={() => {
														template = 'script'
														initContent(lang, script.kind, template)
														script.language = lang
													}}
													disabled={lockedLanguage ||
														(enterpriseLangs.includes(lang) && !$enterpriseLicense)}
												>
													<LanguageIcon {lang} />
													<span class="ml-2 py-2 truncate">{label}</span>
												</Button>
												<svelte:fragment slot="text"
													>{label} is only available with an enterprise license</svelte:fragment
												>
											</Popover>
										{/each}
										<Button
											size="sm"
											variant="border"
											color={template == 'docker' ? 'blue' : 'light'}
											btnClasses={template == 'docker'
												? '!border-2 !bg-blue-50/75 dark:!bg-frost-900/75'
												: 'm-[1px]'}
											disabled={lockedLanguage}
											on:click={() => {
												if (isCloudHosted()) {
													sendUserToast(
														'You cannot use Docker scripts on the multi-tenant platform. Use a dedicated instance or self-host windmill instead.',
														true,
														[
															{
																label: 'Learn more',
																callback: () => {
																	window.open(
																		'https://www.windmill.dev/docs/advanced/docker',
																		'_blank'
																	)
																}
															}
														]
													)
													return
												}
												template = 'docker'
												initContent(Script.language.BASH, script.kind, template)
												script.language = Script.language.BASH
											}}
										>
											<LanguageIcon lang="docker" /><span class="ml-2 py-2">Docker</span>
										</Button>
									</div>
								</Section>

								<Section label="Script kind">
									<svelte:fragment slot="header">
										<Tooltip>
											Tag this script's purpose within flows such that it is available as the
											corresponding action.
										</Tooltip>
									</svelte:fragment>
									<ToggleButtonGroup
										class="h-10"
										selected={script.kind}
										on:selected={({ detail }) => {
											template = 'script'
											script.kind = detail
											initContent(script.language, detail, template)
										}}
									>
										{#each scriptKindOptions as { value, title, desc, documentationLink, Icon }}
											<ToggleButton
												label={title}
												{value}
												tooltip={desc}
												{documentationLink}
												icon={Icon}
												showTooltipIcon={Boolean(desc)}
											/>
										{/each}
									</ToggleButtonGroup>
								</Section>
							</div>
						</TabContent>
						<TabContent value="runtime">
							<div class="flex flex-col gap-8">
								<Section label="Concurrency limits">
									{#if !$enterpriseLicense}
										<Alert
											title="Concurrency limits are going to become an Enterprise Edition feature"
											type="warning"
										/>
									{/if}
									<div class="flex flex-col gap-4">
										<Label label="Max number of executions within the time window">
											<div class="flex flex-row gap-2 max-w-sm">
												<input bind:value={script.concurrent_limit} type="number" />
												<Button
													size="sm"
													color="light"
													on:click={() => {
														script.concurrent_limit = undefined
														script.concurrency_time_window_s = undefined
													}}
													variant="border">Remove Limits</Button
												>
											</div>
										</Label>
										<Label label="Time window in seconds">
											<SecondsInput bind:seconds={script.concurrency_time_window_s} />
										</Label>
									</div>
								</Section>
								<Section label="Worker group tag">
									<svelte:fragment slot="header">
										<Tooltip
											documentationLink="https://www.windmill.dev/docs/core_concepts/worker_groups"
										>
											The script will be executed on a worker configured to accept its worker group
											tag. For instance, you could setup an "highmem", or "gpu" worker group.
										</Tooltip>
									</svelte:fragment>

									{#if $workerTags}
										{#if $workerTags?.length > 0}
											<div class="max-w-sm">
												<select
													bind:value={script.tag}
													on:change={(e) => {
														if (script.tag == '') {
															script.tag = undefined
														}
													}}
												>
													{#if script.tag}
														<option value="">reset to default</option>
													{:else}
														<option value="" disabled selected>Worker Group</option>
													{/if}
													{#each $workerTags ?? [] as tag (tag)}
														<option value={tag}>{tag}</option>
													{/each}
												</select>
											</div>
										{:else}
											<div class="text-sm text-secondary flex flex-row gap-2">
												No custom worker group tag defined on this instance in "Workers {'->'} Assignable
												Tags"
												<a
													href="https://www.windmill.dev/docs/core_concepts/worker_groups"
													target="_blank"
													class="hover:underline"
												>
													<div class="flex flex-row gap-2 items-center">
														See documentation
														<ExternalLink size="12" />
													</div>
												</a>
											</div>
										{/if}
									{:else}
										<Loader2 class="animate-spin" />
									{/if}
								</Section>
								<Section label="Cache">
									<div class="flex gap-2 shrink flex-col">
										<Toggle
											size="sm"
											checked={Boolean(script.cache_ttl)}
											on:change={() => {
												if (script.cache_ttl && script.cache_ttl != undefined) {
													script.cache_ttl = undefined
												} else {
													script.cache_ttl = 300
												}
											}}
											options={{
												right: 'Cache the results for each possible inputs'
											}}
										/>
										<span class="text-secondary text-sm leading-none">
											How long to the keep cache valid
										</span>
										{#if script.cache_ttl}
											<SecondsInput bind:seconds={script.cache_ttl} />
										{:else}
											<SecondsInput disabled />
										{/if}
									</div>
								</Section>
								<Section label="Dedicated Workers">
									<Toggle
										disabled={!$enterpriseLicense ||
											isCloudHosted() ||
											(script.language != Script.language.BUN &&
												script.language != Script.language.PYTHON3)}
										size="sm"
										checked={Boolean(script.dedicated_worker)}
										on:change={() => {
											if (script.dedicated_worker) {
												script.dedicated_worker = undefined
											} else {
												script.dedicated_worker = true
											}
										}}
										options={{
											right: 'Script is run on dedicated workers'
										}}
									/>
									{#if script.dedicated_worker}
										<div class="py-2">
											<Alert type="info" title="Require dedicated workers">
												One worker in a worker group needs to be configured with dedicated worker
												set to: <pre>{$workspaceStore}:{script.path}</pre>
											</Alert>
										</div>
									{/if}
									<svelte:fragment slot="header">
										<Tooltip
											>In this mode, the script is meant to be run on dedicated workers that run the
											script at native speed. Can reach >1500rps per dedicated worker. Only
											available on enterprise edition and for the Bun language.</Tooltip
										>
									</svelte:fragment>
								</Section>
								{#if !isCloudHosted()}
									<Section label="High priority script">
										<Toggle
											disabled={!$enterpriseLicense || isCloudHosted()}
											size="sm"
											checked={script.priority !== undefined && script.priority > 0}
											on:change={() => {
												if (script.priority) {
													script.priority = undefined
												} else {
													script.priority = 100
												}
											}}
											options={{
												right: 'Label as high priority'
											}}
										>
											<svelte:fragment slot="right">
												<input
													type="number"
													class="!w-16 ml-4"
													disabled={script.priority === undefined}
													bind:value={script.priority}
													on:focus
													on:change={() => {
														if (script.priority && script.priority > 100) {
															script.priority = 100
														} else if (script.priority && script.priority < 0) {
															script.priority = 0
														}
													}}
												/>
											</svelte:fragment>
										</Toggle>
										<svelte:fragment slot="header">
											<!-- TODO: Add EE-only badge when we have it -->
											<Tooltip>
												Jobs from script labeled as high priority take precedence over the other
												jobs when in the jobs queue.
												{#if !$enterpriseLicense}This is a feature only available on enterprise
													edition.{/if}
											</Tooltip>
										</svelte:fragment>
									</Section>
								{/if}
								{#if !isCloudHosted()}
									<Section label="Custom env variables">
										<svelte:fragment slot="header">
											<Tooltip
												documentationLink="https://www.windmill.dev/docs/reference#custom-environment-variables"
											>
												Additional static custom env variables to pass to the script.
											</Tooltip>
										</svelte:fragment>
										{#if script.envs && script.envs.length > 0}
											<Alert type="warning" title="Not passed in previews" size="xs">
												Static envs variables are not passed in preview but solely on deployed
												scripts.
											</Alert>
										{/if}
										<div class="w-full mt-2">
											<span class="text-tertiary text-xs pb-2">Format is: `{'<KEY>=<VALUE>'}`</span>
											{#if Array.isArray(script.envs ?? [])}
												{#each script.envs ?? [] as v, i}
													<div class="flex max-w-md mt-1 w-full items-center relative">
														<input type="text" bind:value={v} placeholder="<KEY>=<VALUE>" />
														<button
															transition:fade|local={{ duration: 50 }}
															class="rounded-full p-1 bg-surface/60 duration-200 hover:bg-gray-200 absolute right-2"
															aria-label="Clear"
															on:click={() => {
																script.envs && script.envs.splice(i, 1)
																script.envs = script.envs
															}}
														>
															<X size={14} />
														</button>
													</div>
												{/each}
											{/if}
										</div>
										<div class="flex mt-2">
											<Button
												variant="border"
												color="light"
												size="xs"
												on:click={() => {
													if (script.envs == undefined || !Array.isArray(script.envs)) {
														script.envs = []
													}
													script.envs = script.envs.concat('')
												}}
											>
												<div class="flex flex-row gap-1">
													<Plus size="16" />
													Add item
												</div>
											</Button>
										</div>
									</Section>
								{/if}
							</div>
						</TabContent>
						<TabContent value="ui">
							<div class="mt-4" />
							<ScriptSchema bind:schema={script.schema} />
						</TabContent>
					</div>
				</svelte:fragment>
			</Tabs>
		</DrawerContent>
	</Drawer>

	<div class="flex flex-col h-screen">
		<div class="flex h-full max-h-12 items-center px-4 border-b">
			<div class="justify-between flex gap-2 lg:gap-8 w-full items-center">
				<div class="flex flex-row gap-2">
					<div class="center-center">
						<button
							on:click={async () => {
								metadataOpen = true
							}}
						>
							<LanguageIcon lang={script.language} height={20} />
						</button>
					</div>
					<div class="min-w-64 w-full max-w-md">
						<input
							type="text"
							placeholder="Script summary"
							class="text-sm w-full font-semibold"
							bind:value={script.summary}
						/>
					</div>
				</div>

				<div class="gap-4 flex">
					<div class="flex justify-start w-full">
						<div>
							<button
								on:click={async () => {
									metadataOpen = true
								}}
							>
								<Badge
									color="gray"
									class="center-center !bg-surface-selected !text-tertiary !h-[28px]  !w-[70px] rounded-r-none"
								>
									<Pen size={12} class="mr-2" /> Path
								</Badge>
							</button>
						</div>
						<input
							type="text"
							readonly
							value={script.path}
							size={script.path?.length || 50}
							class="font-mono !text-xs !min-w-[96px] !max-w-[300px] !w-full !h-[28px] !my-0 !py-0 !border-l-0 !rounded-l-none"
							on:focus={({ currentTarget }) => {
								currentTarget.select()
							}}
						/>
					</div>
				</div>

				{#if $enterpriseLicense && initialPath != ''}
					<Awareness />
				{/if}

				<div class="flex flex-row gap-x-1 lg:gap-x-2">
					<Button
						color="light"
						variant="border"
						size="xs"
						on:click={() => {
							if (!savedScript) {
								return
							}
							diffDrawer?.openDrawer()
							diffDrawer?.setDiff({
								mode: 'normal',
								deployed: savedScript,
								draft: savedScript['draft'],
								current: script
							})
						}}
						disabled={!savedScript || !diffDrawer}
					>
						<div class="flex flex-row gap-2 items-center">
							<DiffIcon size={14} />
							Diff
						</div>
					</Button>
					<Button
						color="light"
						variant="border"
						size="xs"
						on:click={() => {
							metadataOpen = true
						}}
						startIcon={{ icon: Settings }}
					>
						Settings
					</Button>
					<Button
						loading={loadingDraft}
						size="xs"
						startIcon={{ icon: Save }}
						on:click={() => saveDraft()}
						disabled={initialPath != '' && !savedScript}
					>
						<span class="hidden sm:flex">
							Save draft&nbsp;<Kbd small isModifier>{getModifierKey()}</Kbd>
						</span>
						<Kbd small>S</Kbd>
					</Button>
					<Button
						loading={loadingSave}
						size="xs"
						startIcon={{ icon: Save }}
						on:click={() => editScript()}
						dropdownItems={computeDropdownItems(initialPath)}
					>
						Deploy
					</Button>
				</div>
			</div>
		</div>

		<ScriptEditor
			collabMode
			edit={initialPath != ''}
			on:format={() => {
				saveDraft()
			}}
			bind:editor
			bind:this={scriptEditor}
			bind:schema={script.schema}
			path={script.path}
			bind:code={script.content}
			lang={script.language}
			{initialArgs}
			kind={script.kind}
			{template}
			tag={script.tag}
		/>
	</div>
{:else}
	Script Builder not available to operators
{/if}
