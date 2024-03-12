<script lang="ts">
	import {
		DraftService,
		NewScript,
		Script,
		ScriptService,
		type NewScriptWithDraft,
		ScheduleService
	} from '$lib/gen'
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { inferArgs } from '$lib/infer'
	import { initialCode } from '$lib/script_helpers'
	import { defaultScripts, enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import {
		cleanValueProperties,
		emptySchema,
		emptyString,
		encodeState,
		formatCron,
		orderedJsonStringify
	} from '$lib/utils'
	import Path from './Path.svelte'
	import ScriptEditor from './ScriptEditor.svelte'
	import { Alert, Badge, Button, Drawer, SecondsInput, Tab, TabContent, Tabs } from './common'
	import LanguageIcon from './common/languageIcons/LanguageIcon.svelte'
	import type { SupportedLanguage } from '$lib/common'
	import Tooltip from './Tooltip.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ErrorHandlerToggleButton from '$lib/components/details/ErrorHandlerToggleButton.svelte'
	import {
		Bug,
		Calendar,
		CheckCircle,
		Code,
		DiffIcon,
		Pen,
		Plus,
		Rocket,
		Save,
		Settings,
		X
	} from 'lucide-svelte'
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
	import type Editor from './Editor.svelte'
	import WorkerTagPicker from './WorkerTagPicker.svelte'
	import MetadataGen from './copilot/MetadataGen.svelte'
	import ScriptSchedules from './ScriptSchedules.svelte'
	import { writable } from 'svelte/store'
	import { type ScriptSchedule, loadScriptSchedule, defaultScriptLanguages } from '$lib/scripts'
	import DefaultScripts from './DefaultScripts.svelte'

	export let script: NewScript
	export let initialPath: string = ''
	export let template: 'docker' | 'script' = 'script'
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

	let scheduleStore = writable<ScriptSchedule>({
		summary: '',
		cron: '0 */5 * * *',
		timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
		args: {},
		enabled: false
	})
	async function loadSchedule() {
		const scheduleRes = await loadScriptSchedule(initialPath, $workspaceStore!)
		if (scheduleRes) {
			scheduleStore.set(scheduleRes)
		}
	}

	$: {
		if (initialPath != '') {
			loadSchedule()
		}
	}

	const enterpriseLangs = ['bigquery', 'snowflake', 'mssql']

	export function setCode(code: string): void {
		editor?.setCode(code)
	}

	$: langs = ($defaultScripts?.order ?? Object.keys(defaultScriptLanguages))
		.map((l) => [defaultScriptLanguages[l], l])
		.filter(
			(x) => $defaultScripts?.hidden == undefined || !$defaultScripts.hidden.includes(x[1])
		) as [string, SupportedLanguage | 'docker'][]

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

	async function createSchedule(path: string) {
		const { cron, timezone, args, enabled, summary } = $scheduleStore

		try {
			await ScheduleService.createSchedule({
				workspace: $workspaceStore!,
				requestBody: {
					path: path,
					schedule: formatCron(cron),
					timezone,
					script_path: path,
					is_flow: false,
					args,
					enabled,
					summary
				}
			})
		} catch (err) {
			sendUserToast(`The primary schedule could not be created: ${err}`, true)
		}
	}

	async function editScript(stay: boolean): Promise<void> {
		loadingSave = true
		try {
			try {
				localStorage.removeItem(script.path)
			} catch (e) {
				console.error('error interacting with local storage', e)
			}
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
					priority: script.priority,
					restart_unless_cancelled: script.restart_unless_cancelled,
					delete_after_use: script.delete_after_use,
					timeout: script.timeout,
					concurrency_key: emptyString(script.concurrency_key) ? undefined : script.concurrency_key
				}
			})

			const { enabled, timezone, args, cron, summary } = $scheduleStore
			const scheduleExists = await ScheduleService.existsSchedule({
				workspace: $workspaceStore ?? '',
				path: script.path
			})

			if (scheduleExists) {
				const schedule = await ScheduleService.getSchedule({
					workspace: $workspaceStore ?? '',
					path: script.path
				})
				if (
					JSON.stringify(schedule.args) != JSON.stringify(args) ||
					schedule.schedule != cron ||
					schedule.timezone != timezone ||
					schedule.summary != summary
				) {
					await ScheduleService.updateSchedule({
						workspace: $workspaceStore ?? '',
						path: script.path,
						requestBody: {
							schedule: formatCron(cron),
							timezone,
							args,
							summary
						}
					})
				}
				if (enabled != schedule.enabled) {
					await ScheduleService.setScheduleEnabled({
						workspace: $workspaceStore ?? '',
						path: script.path,
						requestBody: { enabled }
					})
				}
			} else if (enabled) {
				await createSchedule(script.path)
			}

			savedScript = cloneDeep(script) as NewScriptWithDraft
			history.replaceState(history.state, '', `/scripts/edit/${script.path}`)
			if (stay) {
				script.parent_hash = newHash
			} else {
				goto(`/scripts/get/${newHash}?workspace=${$workspaceStore}`)
			}
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
			try {
				localStorage.removeItem(script.path)
			} catch (e) {
				console.error('error interacting with local storage', e)
			}
			script.schema = script.schema ?? emptySchema()
			try {
				await inferArgs(script.language, script.content, script.schema as any)
			} catch (error) {
				sendUserToast(
					`The main signature was not parsable. This script is considered to be without main function`
				)
			}

			if (initialPath == '' || savedScript?.draft_only) {
				if (savedScript?.draft_only) {
					await ScriptService.deleteScriptByPath({
						workspace: $workspaceStore!,
						path: initialPath
					})
					script.parent_hash = undefined
				}
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
						priority: script.priority,
						restart_unless_cancelled: script.restart_unless_cancelled,
						delete_after_use: script.delete_after_use,
						timeout: script.timeout,
						concurrency_key: emptyString(script.concurrency_key)
							? undefined
							: script.concurrency_key
					}
				})
			}
			await DraftService.createDraft({
				workspace: $workspaceStore!,
				requestBody: {
					path: initialPath == '' || savedScript?.draft_only ? script.path : initialPath,
					typ: 'script',
					value: script
				}
			})

			savedScript = {
				...(initialPath == '' || savedScript?.draft_only
					? { ...cloneDeep(script), draft_only: true }
					: savedScript),
				draft: cloneDeep(script)
			} as NewScriptWithDraft

			if (initialPath == '' || (savedScript?.draft_only && script.path !== initialPath)) {
				initialPath = script.path
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
							label: 'Deploy & Stay here',
							onClick: () => {
								editScript(true)
							}
						},
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

	let selectedTab: 'metadata' | 'runtime' | 'ui' | 'schedule' = 'metadata'
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
				<Tab value="schedule" active={$scheduleStore.enabled}>Schedule</Tab>
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
											<MetadataGen
												label="Summary"
												bind:content={script.summary}
												lang={script.language}
												code={script.content}
												promptConfigName="summary"
												generateOnAppear
												on:change={() => {
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
												elementProps={{
													type: 'text',
													placeholder: 'Short summary to be displayed when listed'
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
											<MetadataGen
												bind:content={script.description}
												lang={script.language}
												code={script.content}
												promptConfigName="description"
												elementType="textarea"
												elementProps={{
													placeholder: 'Description displayed'
												}}
											/>
										</Label>
									</div>
								</Section>

								<Section label="Language">
									<svelte:fragment slot="action"><DefaultScripts /></svelte:fragment>
									{#if lockedLanguage}
										<div class="text-sm text-tertiary italic mb-2">
											As a forked script, the language '{script.language}' cannot be modified.
										</div>
									{/if}
									<div class=" grid grid-cols-3 gap-2">
										{#each langs as [label, lang] (lang)}
											{@const isPicked =
												(lang == script.language && template == 'script') ||
												(template == 'docker' && lang == 'docker')}
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
														if (lang == 'docker') {
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
														} else {
															template = 'script'
														}
														let language = lang == 'docker' ? Script.language.BASH : lang
														//
														initContent(language, script.kind, template)
														script.language = language
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
								<Section label="Concurrency limits" eeOnly>
									<div class="flex flex-col gap-4">
										<Label label="Max number of executions within the time window">
											<div class="flex flex-row gap-2 max-w-sm">
												<input
													disabled={!$enterpriseLicense}
													bind:value={script.concurrent_limit}
													type="number"
												/>
												<Button
													size="sm"
													color="light"
													on:click={() => {
														script.concurrent_limit = undefined
														script.concurrency_time_window_s = undefined
														script.concurrency_key = undefined
													}}
													variant="border">Remove Limits</Button
												>
											</div>
										</Label>
										<Label label="Time window in seconds">
											<SecondsInput
												disabled={!$enterpriseLicense}
												bind:seconds={script.concurrency_time_window_s}
											/>
										</Label>
										<Label label="Custom concurrency key">
											<input
												type="text"
												autofocus
												bind:value={script.concurrency_key}
												placeholder={`$workspace/script/${script.path}-$args[foo]`}
											/>
											<Tooltip
												>Concurrency keys are global, you can have them be workspace specific using
												the variable `$workspace`. You can also use an argument's value using
												`$args[name_of_arg]`</Tooltip
											>
										</Label>
									</div>
								</Section>
								<Section label="Worker Group Tag (Queue)">
									<svelte:fragment slot="header">
										<Tooltip
											documentationLink="https://www.windmill.dev/docs/core_concepts/worker_groups"
										>
											The script will be executed on a worker configured to listen to this worker
											group tag (queue). For instance, you could setup an "highmem", or "gpu" tag.
										</Tooltip>
									</svelte:fragment>
									<WorkerTagPicker bind:tag={script.tag} />
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
								<Section label="Timeout">
									<div class="flex gap-2 shrink flex-col">
										<Toggle
											size="sm"
											checked={Boolean(script.timeout)}
											on:change={() => {
												if (script.timeout && script.timeout != undefined) {
													script.timeout = undefined
												} else {
													script.timeout = 300
												}
											}}
											options={{
												right: 'Add a custom timeout for this script'
											}}
										/>
										<span class="text-secondary text-sm leading-none"> Timeout duration </span>
										{#if script.timeout}
											<SecondsInput bind:seconds={script.timeout} />
										{:else}
											<SecondsInput disabled />
										{/if}
									</div>
								</Section>
								<Section label="Perpetual Script">
									<div class="flex gap-2 shrink flex-col">
										<Toggle
											size="sm"
											checked={Boolean(script.restart_unless_cancelled)}
											on:change={() => {
												if (script.restart_unless_cancelled) {
													script.restart_unless_cancelled = undefined
												} else {
													script.restart_unless_cancelled = true
												}
											}}
											options={{
												right: 'Restart upon ending unless cancelled'
											}}
										/>
									</div>
								</Section>
								<Section label="Dedicated Workers" eeOnly>
									<Toggle
										disabled={!$enterpriseLicense ||
											isCloudHosted() ||
											(script.language != Script.language.BUN &&
												script.language != Script.language.PYTHON3 &&
												script.language != Script.language.DENO)}
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
											available on enterprise edition and for Python3, Deno and Bun. For other
											languages, the efficiency is already on par with deidcated workers since they
											do not spawn a full runtime</Tooltip
										>
									</svelte:fragment>
								</Section>
								<Section label="Delete after use">
									<svelte:fragment slot="header">
										<Tooltip>
											WARNING: This settings ONLY applies to synchronous webhooks or when the script
											is used within a flow. If used individually, this script must be triggered
											using a synchronous endpoint to have the desired effect.
											<br />
											<br />
											The logs, arguments and results of the job will be completely deleted from Windmill
											once it is complete and the result has been returned.
											<br />
											<br />
											The deletion is irreversible.
											{#if !$enterpriseLicense}
												<br />
												<br />
												This option is only available on Windmill Enterprise Edition.
											{/if}
										</Tooltip>
									</svelte:fragment>
									<div class="flex gap-2 shrink flex-col">
										<Toggle
											disabled={!$enterpriseLicense}
											size="sm"
											checked={Boolean(script.delete_after_use)}
											on:change={() => {
												if (script.delete_after_use) {
													script.delete_after_use = undefined
												} else {
													script.delete_after_use = true
												}
											}}
											options={{
												right: 'Delete logs, arguments and results after use'
											}}
										/>
									</div>
								</Section>
								{#if !isCloudHosted()}
									<Section label="High priority script" eeOnly>
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
												documentationLink="https://www.windmill.dev/docs/script_editor/custom_environment_variables"
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
						<TabContent value="schedule">
							<ScriptSchedules {initialPath} schema={script.schema} schedule={scheduleStore} />
						</TabContent>
					</div>
				</svelte:fragment>
			</Tabs>
		</DrawerContent>
	</Drawer>

	<div class="flex flex-col h-screen">
		<div class="flex h-12 items-center px-4">
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
					<div class="min-w-32 lg:min-w-64 w-full max-w-md">
						<input
							type="text"
							placeholder="Script summary"
							class="text-sm w-full font-semibold"
							bind:value={script.summary}
						/>
					</div>
				</div>

				<div class="gap-4 flex">
					{#if $scheduleStore.enabled}
						<Button
							btnClasses="hidden lg:inline-flex"
							startIcon={{ icon: Calendar }}
							variant="contained"
							color="light"
							size="xs"
							on:click={async () => {
								metadataOpen = true
								selectedTab = 'schedule'
							}}
						>
							{$scheduleStore.cron ?? ''}
						</Button>
					{/if}
					<div class="flex justify-start w-full border rounded-md overflow-hidden">
						<div>
							<button
								on:click={async () => {
									metadataOpen = true
								}}
							>
								<Badge
									color="gray"
									class="center-center !bg-surface-secondary !text-tertiary !h-[28px]  !w-[70px] rounded-none hover:!bg-surface-hover transition-all"
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
							class="font-mono !text-xs !min-w-[96px] !max-w-[300px] !w-full !h-[28px] !my-0 !py-0 !border-l-0 !rounded-l-none !border-0 !shadow-none"
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
							<span class="hidden lg:flex"> Diff </span>
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
						<span class="hidden lg:flex"> Settings </span>
					</Button>
					<Button
						loading={loadingDraft}
						size="xs"
						startIcon={{ icon: Save }}
						on:click={() => saveDraft()}
						disabled={initialPath != '' && !savedScript}
						shortCut={{
							key: 'S'
						}}
					>
						<span class="hidden lg:flex"> Draft </span>
					</Button>
					<Button
						loading={loadingSave}
						size="xs"
						startIcon={{ icon: Save }}
						on:click={() => editScript(false)}
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
