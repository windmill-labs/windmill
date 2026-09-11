<script lang="ts">
	import { Alert, Badge, Button, ButtonType, Tab, Tabs } from '$lib/components/common'
	import {
		clearPageDrawerAnchor,
		setPageDrawerAnchor
	} from '$lib/components/sessions/pageDrawerSession'
	import { SCHEDULES_PATH } from '$lib/components/sessions/previewPaths'
	import TriggerAdvancedBadges from '../TriggerAdvancedBadges.svelte'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import CronInput from '$lib/components/CronInput.svelte'
	import Path from '$lib/components/Path.svelte'
	import LabelsInput from '$lib/components/LabelsInput.svelte'
	import Required from '$lib/components/Required.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import PipelineLockedRunnableInfo from '$lib/components/triggers/PipelineLockedRunnableInfo.svelte'
	import ErrorOrRecoveryHandler from '$lib/components/ErrorOrRecoveryHandler.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import {
		FlowService,
		ScheduleService,
		type Script,
		ScriptService,
		type Flow,
		type Retry,
		type Schedule,
		type ErrorHandler
	} from '$lib/gen'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { canWrite, emptyString, sendUserToast, cronV1toV2 } from '$lib/utils'
	import { base } from '$lib/base'
	import Section from '$lib/components/Section.svelte'
	import { List, Loader2, Save, AlertTriangle } from 'lucide-svelte'
	import autosize from '$lib/autosize'
	import TriggerEditorToolbar from '$lib/components/triggers/TriggerEditorToolbar.svelte'
	import { writeScheduleCfg } from '$lib/components/flows/scheduleUtils'
	import DateTimeInput from '$lib/components/DateTimeInput.svelte'
	import FlowRetries from '$lib/components/flows/content/FlowRetries.svelte'
	import Label from '$lib/components/Label.svelte'
	import WorkerTagPicker from '$lib/components/WorkerTagPicker.svelte'
	import { runScheduleNow } from '../scheduled/utils'
	import { handleConfigChange } from '../utils'
	import { withForkConflictRetry } from '$lib/utils/forkConflict'
	import LocalDraftBanner from '$lib/components/LocalDraftBanner.svelte'
	import DraftConflictAlert from '$lib/components/DraftConflictAlert.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { twMerge } from 'tailwind-merge'
	import PermissionedAsLine from '../PermissionedAsLine.svelte'
	import { getTriggerWorkspace } from '$lib/components/triggers/triggerWorkspace'
	import { useActingUser } from '$lib/actingUser.svelte'
	import { untrack } from 'svelte'
	import { resource } from 'runed'
	import { onUserInput } from '$lib/userDraftEditGate'
	import { isTemporaryPath, newItemPath, useItem, type ItemAdapter } from '$lib/itemStore.svelte'
	import {
		newScheduleCfg,
		normalizeScheduleCfg,
		scheduleCfgOf,
		scheduleFormOf,
		type NewScheduleOptions,
		type ScheduleCfg
	} from './scheduleCfg'

	let {
		useDrawer = true,
		hideTarget = false,
		docDescription = undefined,
		allowDraft = false,
		draftSchema = undefined,
		customLabel = undefined,
		isDeployed = false,
		onUpdate = undefined,
		onConfigChange = undefined,
		onDelete = undefined,
		onReset = undefined,
		trigger = undefined
	} = $props()

	let optionTabSelected:
		| 'error_handler'
		| 'recovery_handler'
		| 'success_handler'
		| 'retries'
		| 'dynamic_skip' = $state('error_handler')
	let initialPath = $state('')
	/** `edit`: a schedule at `initialPath`, read from the server. `fixed`: one whose config the
	 * caller handed in (a runnable's trigger panel). `new`: being created. */
	let mode = $state<'edit' | 'fixed' | 'new'>('edit')
	let schedule: string = $state('0 0 12 * *')
	let cronVersion: string = $state('v2')
	let isLatestCron = $state(true)
	let initialCronVersion: string = $state('v2')
	let initialSchedule: string
	let timezone: string = $state(Intl.DateTimeFormat().resolvedOptions().timeZone)
	let paused_until: string | undefined = $state(undefined)
	let itemKind: 'flow' | 'script' = $state('script')
	let is_flow: boolean = $derived.by(() => itemKind === 'flow')
	let errorHandleritemKind: 'flow' | 'script' = $state('script')
	let wsErrorHandlerMuted: boolean = $state(false)
	let errorHandlerPath: string | undefined = $state(undefined)
	let errorHandlerSelected: ErrorHandler = $state('slack')
	let errorHandlerExtraArgs: Record<string, any> = $state({})
	let recoveryHandlerPath: string | undefined = $state(undefined)
	let recoveryHandlerSelected: ErrorHandler = $state('slack')
	let recoveryHandlerItemKind: 'flow' | 'script' = $state('script')
	let recoveryHandlerExtraArgs: Record<string, any> = $state({})
	let successHandlerPath: string | undefined = $state(undefined)
	let successHandlerSelected: ErrorHandler = $state('slack')
	let successHandlerItemKind: 'flow' | 'script' = $state('script')
	let successHandlerExtraArgs: Record<string, any> = $state({})
	let failedTimes = $state(1)
	let failedExact = $state(false)
	let recoveredTimes = $state(1)
	let retry: Retry | undefined = $state(undefined)
	let dynamicSkipPath: string | undefined = $state(undefined)
	let script_path = $state('')
	let initialScriptPath = $state('')
	// When non-empty, the drawer was opened from the pipeline editor for an
	// already-bound script. We swap the runnable ScriptPicker for a read-only
	// viewer so the trigger can't be silently reassigned off the pipeline.
	let fixedScriptPath = $state('')
	let args: Record<string, any> = $state({})
	let showLoading = $state(false)
	let extraPerms: Record<string, boolean> = $state({})
	// Path the permissions above were loaded for — the verdict is about the schedule as
	// stored, not about a rename being typed into the form. `undefined` until a config has
	// been loaded, when there is no deployed schedule to deny access to.
	let permsPath: string | undefined = $state(undefined)
	let initNewPath = $state(false)
	let path: string = $state('')
	let enabled: boolean = $state(false)
	let pathError = $state('')
	let summary = $state('')
	let labels: string[] | undefined = $state(undefined)
	let description = $state('')
	let no_flow_overlap = $state(false)
	let tag: string | undefined = $state(undefined)
	let validCRON = $state(true)
	let isValid = $state(true)
	let allowSchedule = $derived(isValid && validCRON && script_path != '')
	let permissionedAs = $state<string | undefined>(undefined)
	let selectedPermissionedAs = $state<string | undefined>(undefined)
	let preservePermissionedAs = $state(false)

	const triggerWs = getTriggerWorkspace()
	const wsId = $derived(triggerWs?.() ?? $workspaceStore)
	// `undefined` while the lookup is in flight or after it failed; the checks below then
	// refuse rather than fall back to rights that belong to another workspace.
	const acting = useActingUser(() => wsId)
	const actingUser = $derived(acting.current)
	const can_write = $derived(
		permsPath === undefined ? true : canWrite(permsPath, extraPerms, actingUser)
	)
	// Editing the runnable is closed to operators, and an unresolved acting user is no
	// evidence that this one isn't.
	const canEditRunnable = $derived(actingUser !== undefined && !actingUser.operator)
	const saveDisabled = $derived(
		!allowSchedule ||
			pathError != '' ||
			emptyString(script_path) ||
			(errorHandlerSelected == 'slack' &&
				!emptyString(errorHandlerPath) &&
				emptyString(errorHandlerExtraArgs['channel'])) ||
			!can_write
	)
	// Carry the acting workspace onto "create from template" routes when a
	// session override is set, so the script is created in the session workspace.
	const wsParam = $derived(triggerWs?.() ? `&workspace=${encodeURIComponent(wsId!)}` : '')
	const scheduleCfg = $derived.by(getScheduleCfg)

	// The item the form edits. A schedule read from the server is keyed by its path; a new one,
	// or one whose config the caller supplied, by a temporary path that is never persisted.
	let itemPath: string | undefined = $state(undefined)
	let session = $state(0)
	let fixedTemplate: ScheduleCfg | undefined = undefined
	const newTemplates = new Map<string, NewScheduleOptions>()
	// Temporary keys holding a config the caller supplied for a schedule that already exists:
	// saving one updates that schedule rather than creating it.
	const standsForDeployed = new Set<string>()

	const scheduleAdapter: ItemAdapter<ScheduleCfg> = {
		settles: true,
		async load({ workspace, path }) {
			if (isTemporaryPath(path)) {
				const opts = newTemplates.get(path)
				if (!opts) throw new Error('No template for this schedule')
				return { template: await newScheduleCfg(opts) }
			}
			try {
				const s = await ScheduleService.getSchedule({ workspace, path, getDraft: true })
				// Drafts are saved in the form's config shape, over the deployed fields.
				const { draft, draft_saved_at, no_deployed, ...deployedSchedule } = s as any
				return {
					deployed: no_deployed ? undefined : normalizeScheduleCfg(deployedSchedule),
					draft: draft ? normalizeScheduleCfg({ ...deployedSchedule, ...draft }) : undefined,
					draftSavedAt: draft_saved_at
				}
			} catch (err) {
				sendUserToast(`Could not load schedule: ${err}`, true)
				throw err
			}
		},
		async write({ workspace, path, value, deployed }) {
			await writeScheduleCfg(
				value,
				deployed !== undefined || standsForDeployed.has(path),
				workspace
			)
		}
	}

	const item = useItem<ScheduleCfg>(
		'trigger_schedule',
		() => ({ workspace: wsId, path: itemPath, session, template: fixedTemplate }),
		scheduleAdapter
	)

	// A schedule read from the server is created, not updated, while it only exists as a draft;
	// one this editor created is edited from then on, as the drawer can outlive the create.
	const edit = $derived(
		mode === 'fixed' || item.origin === 'deployed' || (mode === 'edit' && item.origin !== 'draft')
	)
	// Where the schedule is stored: the item's own path once it has one.
	const schedulePath = $derived(
		item.current && !isTemporaryPath(item.key.path) ? item.key.path : initialPath
	)
	const hasBaseline = $derived(
		item.loaded && (item.origin === 'deployed' || item.origin === 'draft')
	)

	// Which item's value the form holds, and as of which revision. Edits only flow back into
	// the item the form was filled from, never into one it has not caught up with yet.
	let hydratedHandle: unknown = $state(undefined)
	let hydratedRevision = $state(0)
	const drawerLoading = $derived(
		!item.loaded || item.value === undefined || hydratedHandle !== item.current
	)

	$effect(() => {
		const current = item.current
		const revision = item.revision
		const value = item.value
		if (!current || !item.loaded || value === undefined) return
		untrack(() => {
			if (current === hydratedHandle && revision === hydratedRevision) return
			hydrate($state.snapshot(value) as ScheduleCfg)
			hydratedHandle = current
			hydratedRevision = revision
		})
	})

	$effect(() => {
		const cfg = $state.snapshot(scheduleCfg) as ScheduleCfg
		untrack(() => {
			if (hydratedHandle !== item.current || hydratedRevision !== item.revision) return
			if (item.value === undefined) return
			item.value = cfg
		})
	})

	// The form's own settling is not an edit; the first input is. These editors are mounted by
	// the list page, so input while the drawer loads is the click that opened it.
	onUserInput(() => {
		if (!drawerLoading) item.markEdited()
	})

	$effect(() => {
		if (!drawerLoading) {
			showLoading = false
			return
		}
		// Do not show the loading spinner for the first 100ms
		const timeout = setTimeout(() => (showLoading = true), 100)
		return () => clearTimeout(timeout)
	})

	// Set by Save and cleared by opening anything: closes the drawer once the item on screen has
	// landed with nothing left unsaved, so an edit typed during the write stays in front of you.
	let closeOnSettle = $state(false)
	$effect(() => {
		if (!closeOnSettle || item.busy) return
		untrack(() => {
			closeOnSettle = false
			if (item.status === 'idle' && !item.dirty) drawer?.closeDrawer()
		})
	})

	// A draft-only schedule is gone once its draft is discarded.
	$effect(() => {
		if (!item.removed) return
		untrack(() => {
			onUpdate?.(schedulePath)
			drawer?.closeDrawer()
		})
	})

	export async function openEdit(
		ePath: string,
		isFlow: boolean,
		defaultCfg?: Record<string, any>,
		fixedScriptPath_?: string
	) {
		acting.forgetFailures()
		closeOnSettle = false
		drawer?.openDrawer()
		setPageDrawerAnchor(SCHEDULES_PATH, ePath)
		initialPath = ePath
		itemKind = isFlow ? 'flow' : 'script'
		fixedScriptPath = fixedScriptPath_ ?? ''
		if (defaultCfg) {
			mode = 'fixed'
			fixedTemplate = normalizeScheduleCfg({ ...defaultCfg, path: defaultCfg.path ?? ePath })
			itemPath = newItemPath()
			standsForDeployed.add(itemPath)
		} else {
			mode = 'edit'
			fixedTemplate = undefined
			itemPath = ePath
		}
		session++
	}

	export async function openNew(
		nis_flow: boolean,
		initial_script_path?: string,
		defaultValues?: Schedule,
		schedule_path?: string,
		fixedScriptPath_?: string,
		opts: { getDraft?: boolean } = {}
	) {
		acting.forgetFailures()
		closeOnSettle = false
		drawer?.openDrawer()
		mode = 'new'
		if (schedule_path) initNewPath = true
		initialScriptPath = initial_script_path ?? ''
		fixedScriptPath = fixedScriptPath_ ?? ''
		initialPath = schedule_path
			? ''
			: (defaultValues?.path ?? (trigger?.isPrimary ? initialScriptPath : ''))
		fixedTemplate = undefined
		const temporary = newItemPath()
		newTemplates.set(temporary, {
			workspace: wsId!,
			isFlow: nis_flow,
			initialScriptPath,
			defaultValues,
			schedulePath: schedule_path,
			getDraft: opts.getDraft ?? true,
			isPrimary: !!trigger?.isPrimary
		})
		itemPath = temporary
		session++
	}

	/** Fill the form from `cfg`. Synchronous, so the form never holds half of one config. */
	function hydrate(cfg: ScheduleCfg): void {
		const f = scheduleFormOf(cfg)
		path = f.path
		cronVersion = f.cronVersion
		initialCronVersion = cronVersion
		isLatestCron = cronVersion == 'v2'
		enabled = f.enabled
		schedule = f.schedule
		initialSchedule = schedule
		timezone = f.timezone
		paused_until = f.paused_until
		showPauseUntil = paused_until !== undefined
		summary = f.summary
		labels = f.labels
		description = f.description
		script_path = f.script_path
		itemKind = f.itemKind
		no_flow_overlap = f.no_flow_overlap
		wsErrorHandlerMuted = f.wsErrorHandlerMuted
		retry = f.retry
		errorHandleritemKind = f.errorHandleritemKind
		errorHandlerPath = f.errorHandlerPath
		failedTimes = f.failedTimes
		failedExact = f.failedExact
		errorHandlerExtraArgs = f.errorHandlerExtraArgs
		errorHandlerSelected = f.errorHandlerSelected
		recoveryHandlerItemKind = f.recoveryHandlerItemKind
		recoveryHandlerPath = f.recoveryHandlerPath
		recoveredTimes = f.recoveredTimes
		recoveryHandlerExtraArgs = f.recoveryHandlerExtraArgs
		recoveryHandlerSelected = f.recoveryHandlerSelected
		successHandlerItemKind = f.successHandlerItemKind
		successHandlerPath = f.successHandlerPath
		successHandlerExtraArgs = f.successHandlerExtraArgs
		successHandlerSelected = f.successHandlerSelected
		dynamicSkipPath = f.dynamicSkipPath
		args = f.args
		extraPerms = f.extraPerms
		// A schedule being created has no stored permissions to judge against.
		permsPath = mode === 'new' ? undefined : f.path
		tag = f.tag
		permissionedAs = cfg.permissioned_as
		selectedPermissionedAs = f.selectedPermissionedAs
		preservePermissionedAs = f.preservePermissionedAs
	}

	// set isValid to true when a script/flow without any properties is selected
	$effect(() => {
		setDefaultValid(draftSchema ?? runnable?.schema)
	})

	function setDefaultValid(schema: Record<string, any> | undefined) {
		if (!isValid) {
			let isEmpty = schema?.properties == undefined || Object.keys(schema.properties).length === 0
			if (isEmpty) {
				isValid = true
			}
		}
	}

	const runnableResource = resource(
		[() => wsId, () => script_path, () => is_flow],
		async ([ws, p, flow]): Promise<Script | Flow | undefined> => {
			if (!ws || !p) return undefined
			try {
				return flow
					? await FlowService.getFlowByPath({ workspace: ws, path: p })
					: await ScriptService.getScriptByPath({ workspace: ws, path: p })
			} catch {
				return undefined
			}
		}
	)
	// Only the runnable the picker names: the previous one's schema would fill `args` in.
	const loading = $derived(runnableResource.loading)
	const runnable = $derived(loading ? undefined : runnableResource.current)

	async function saveAsDefaultErrorHandler(overrideExisting: boolean) {
		if (!$enterpriseLicense) {
			sendUserToast(`Setting default error handler is an enterprise edition feature`, true)
			return
		}
		if (wsId) {
			await ScheduleService.setDefaultErrorOrRecoveryHandler({
				workspace: wsId!,
				requestBody: {
					handler_type: 'error',
					override_existing: overrideExisting,
					path:
						errorHandlerPath == undefined
							? undefined
							: `${errorHandleritemKind}/${errorHandlerPath}`,
					extra_args: errorHandlerExtraArgs,
					number_of_occurence: failedTimes,
					number_of_occurence_exact: failedExact,
					workspace_handler_muted: wsErrorHandlerMuted
				}
			})
			if (errorHandlerPath !== undefined) {
				sendUserToast(`Default error handler saved to ${errorHandlerPath}`, false)
			} else {
				sendUserToast(`Default error handler reset`, false)
			}
		}
	}

	async function saveAsDefaultRecoveryHandler(overrideExisting: boolean) {
		if (!$enterpriseLicense) {
			sendUserToast(`Setting default recovery handler is an enterprise edition feature`, true)
			return
		}
		if (wsId) {
			await ScheduleService.setDefaultErrorOrRecoveryHandler({
				workspace: wsId!,
				requestBody: {
					handler_type: 'recovery',
					override_existing: overrideExisting,
					path:
						recoveryHandlerPath === undefined
							? undefined
							: `${recoveryHandlerItemKind}/${recoveryHandlerPath}`,
					extra_args: recoveryHandlerExtraArgs,
					number_of_occurence: recoveredTimes
				}
			})
			if (recoveryHandlerPath !== undefined) {
				sendUserToast(`Default recovery handler saved to ${recoveryHandlerPath}`, false)
			} else {
				sendUserToast(`Default recovery handler reset`, false)
			}
		}
	}

	async function saveAsDefaultSuccessHandler(overrideExisting: boolean) {
		if (!$enterpriseLicense) {
			sendUserToast(`Setting default success handler is an enterprise edition feature`, true)
			return
		}
		if (wsId) {
			await ScheduleService.setDefaultErrorOrRecoveryHandler({
				workspace: wsId!,
				requestBody: {
					handler_type: 'success',
					override_existing: overrideExisting,
					path:
						successHandlerPath === undefined
							? undefined
							: `${successHandlerItemKind}/${successHandlerPath}`,
					extra_args: successHandlerExtraArgs,
					number_of_occurence: recoveredTimes
				}
			})
			if (successHandlerPath !== undefined) {
				sendUserToast(`Default success handler saved to ${successHandlerPath}`, false)
			} else {
				sendUserToast(`Default success handler reset`, false)
			}
		}
	}

	async function scheduleScript(): Promise<void> {
		const created = !edit
		closeOnSettle = true
		const outcome = await item.save()
		if (!outcome.ok) {
			sendUserToast(outcome.error, true)
			return
		}
		sendUserToast(`Schedule ${outcome.path} ${created ? 'created' : 'updated'}`)
		onUpdate?.(outcome.path)
	}

	let drawer: Drawer | undefined = $state()

	let pathC: Path | undefined = $state()
	let dirtyPath = $state(false)

	let showPauseUntil = $state(false)
	$effect(() => {
		!showPauseUntil && (paused_until = undefined)
	})

	function onVersionChange() {
		cronVersion = isLatestCron ? 'v2' : 'v1'
		if (cronVersion === 'v2' && initialCronVersion === 'v1') {
			// switches day-of-week from v1 -> v2
			schedule = cronV1toV2(schedule)
		} else if (
			cronVersion === 'v1' &&
			initialCronVersion === 'v1' &&
			schedule !== initialSchedule
		) {
			// revert back to original
			schedule = initialSchedule
		}
	}

	function getScheduleCfg(): ScheduleCfg {
		return scheduleCfgOf({
			path,
			cronVersion,
			schedule,
			timezone,
			paused_until,
			enabled,
			summary,
			labels,
			description,
			script_path,
			itemKind,
			no_flow_overlap,
			wsErrorHandlerMuted,
			retry,
			errorHandleritemKind,
			errorHandlerPath,
			failedTimes,
			failedExact,
			errorHandlerExtraArgs,
			errorHandlerSelected,
			recoveryHandlerItemKind,
			recoveryHandlerPath,
			recoveredTimes,
			recoveryHandlerExtraArgs,
			recoveryHandlerSelected,
			successHandlerItemKind,
			successHandlerPath,
			successHandlerExtraArgs,
			successHandlerSelected,
			dynamicSkipPath,
			args,
			extraPerms,
			tag,
			selectedPermissionedAs,
			preservePermissionedAs
		})
	}

	async function handleToggleEnabled(nEnabled: boolean) {
		enabled = nEnabled
		if (trigger?.draftConfig) return
		const target = schedulePath
		const workspace = wsId ?? ''
		// Queued behind any save of this schedule; on refusal the item gives the field back and
		// the form re-reads it.
		const outcome = await item.patch({ enabled: nEnabled }, async () => {
			const ok = await withForkConflictRetry(
				(force) =>
					ScheduleService.setScheduleEnabled({
						path: target,
						workspace,
						requestBody: { enabled: nEnabled, force }
					}),
				'schedule'
			)
			if (!ok) throw new Error(`Could not ${nEnabled ? 'enable' : 'disable'} ${target}`)
		})
		if (!outcome.ok) return
		sendUserToast(`${nEnabled ? 'enabled' : 'disabled'} schedule ${target}`)
		onUpdate?.(target)
	}

	$effect(() => {
		if (!drawerLoading) {
			handleConfigChange(scheduleCfg, item.deployed, saveDisabled, edit, onConfigChange)
		}
	})
</script>

<!-- {JSON.stringify({ allowSchedule, path: script_path, validCRON, isValid })} -->
{#snippet saveButton()}
	{#if !drawerLoading}
		<TriggerEditorToolbar
			triggerPath={schedulePath}
			triggerKind="schedule"
			{trigger}
			permissions={drawerLoading || !can_write ? 'none' : 'create'}
			saveDisabled={saveDisabled || item.busy}
			mode={enabled ? 'enabled' : 'disabled'}
			{allowDraft}
			{edit}
			isLoading={item.busy}
			onUpdate={scheduleScript}
			{onReset}
			{onDelete}
			onToggleMode={(mode) => handleToggleEnabled(mode === 'enabled')}
			{isDeployed}
			disableSuspendedMode
		>
			{#snippet extra()}
				{#if !drawerLoading && edit}
					<div class="mr-12 flex flex-row gap-3">
						<Button
							size="sm"
							variant="default"
							startIcon={{ icon: List }}
							disabled={!allowSchedule || pathError != '' || emptyString(script_path)}
							href={`${base}/runs/?schedule_path=${path}&job_trigger_kind=schedule&show_future_jobs=true`}
						>
							View runs
						</Button>
						<Button
							size="sm"
							variant="default"
							disabled={!allowSchedule || pathError != '' || emptyString(script_path)}
							on:click={() => {
								runScheduleNow(script_path, path, is_flow, wsId!)
							}}
						>
							Run now
						</Button>
					</div>
				{/if}
			{/snippet}
		</TriggerEditorToolbar>
	{/if}
{/snippet}

{#snippet content()}
	{#if drawerLoading}
		{#if showLoading}
			<Loader2 class="animate-spin" />
		{/if}
	{:else}
		{#if item.status === 'conflicted'}
			<DraftConflictAlert
				onReload={() => item.resolveConflict('reload')}
				onOverwrite={() => item.resolveConflict('overwrite')}
			/>
		{/if}
		<PermissionedAsLine
			{permissionedAs}
			{path}
			onPermissionedAsChange={(pa, preserve) => {
				selectedPermissionedAs = pa
				preservePermissionedAs = preserve
			}}
		/>
		<div class="flex flex-col gap-8">
			<Section headless>
				<div class="flex flex-col gap-6">
					<label class="flex flex-col gap-1">
						<span class="text-xs font-semibold text-emphasis">Summary</span>
						<!-- svelte-ignore a11y_autofocus -->
						<TextInput
							inputProps={{
								autofocus: true,
								type: 'text',
								placeholder: 'Short summary to be displayed when listed',
								disabled: !can_write,
								onkeyup: () => {
									if (!edit && summary?.length > 0 && !dirtyPath) {
										pathC?.setName(
											summary
												.toLowerCase()
												.replace(/[^a-z0-9_]/g, '_')
												.replace(/-+/g, '_')
												.replace(/^-|-$/g, '')
										)
									}
								}
							}}
							bind:value={summary}
						/>
					</label>
					<LabelsInput bind:labels class="-mt-4" />

					<div class="flex flex-col gap-1">
						<label for="path" class="text-xs font-semibold text-emphasis">Path</label>
						{#if !edit && !trigger?.isPrimary}
							<Path
								workspaceOverride={wsId}
								bind:dirty={dirtyPath}
								bind:this={pathC}
								checkInitialPathExistence={!edit}
								bind:error={pathError}
								bind:path
								{initialPath}
								namePlaceholder="schedule"
								kind="schedule"
								disableEditing={!can_write}
								actingUser={actingUser ?? null}
							/>
						{:else}
							<div class="flex justify-start w-full">
								<Badge
									color="gray"
									class={twMerge(
										'center-center !bg-surface-secondary !text-secondary rounded-r-none border',
										ButtonType.UnifiedMinHeightClasses['md']
									)}
								>
									Schedule path (not editable)
								</Badge>
								<input
									type="text"
									readonly
									value={path}
									size={path?.length || 50}
									class={twMerge(
										'font-mono !text-2xs grow shrink overflow-x-auto !py-0 !border-l-0 !rounded-l-none',
										ButtonType.UnifiedMinHeightClasses['md']
									)}
									onfocus={({ currentTarget }) => {
										currentTarget.select()
									}}
								/>
								<!-- <span class="font-mono text-sm break-all">{path}</span> -->
							</div>
						{/if}
					</div>

					<label class="flex flex-col gap-1">
						<span class="text-xs font-semibold text-emphasis">Description</span>
						<textarea
							rows="4"
							use:autosize
							bind:value={description}
							placeholder="What this schedule does and how to use it"
							disabled={!can_write}
						></textarea>
					</label>
				</div>
			</Section>

			<Section label="Schedule">
				{#snippet header()}
					{#if cronVersion === 'v1'}
						<Tooltip>Schedules use CRON syntax. Seconds are mandatory.</Tooltip>
					{:else}
						<Tooltip
							>Schedules use <a
								href="https://www.windmill.dev/docs/core_concepts/scheduling#cron-syntax"
								>extended CRON syntax</a
							>.</Tooltip
						>
					{/if}
				{/snippet}
				<div class="flex flex-col gap-6">
					{#if initialCronVersion !== 'v2'}
						<div class="flex flex-row">
							<AlertTriangle color="orange" class="mr-2" size={16} />
							<Toggle
								options={{
									right: 'enable latest Cron syntax',
									rightTooltip:
										'The latest Cron syntax is more flexible and allows for more complex schedules. See the documentation for more information.',
									rightDocumentationLink:
										'https://www.windmill.dev/docs/core_concepts/scheduling#cron-syntax'
								}}
								size="xs"
								bind:checked={isLatestCron}
								on:change={onVersionChange}
								disabled={!can_write}
							/>
						</div>
					{/if}
					<CronInput
						disabled={!can_write}
						bind:schedule
						bind:timezone
						bind:validCRON
						bind:cronVersion
					/>
					<div class="flex flex-col gap-1">
						<Toggle
							options={{
								right: 'Pause schedule until...',
								rightTooltip:
									'Pausing the schedule will program the next job to run as if the schedule starts at the time the pause is lifted, instead of now.'
							}}
							bind:checked={showPauseUntil}
							disabled={!can_write}
						/>
						{#if showPauseUntil}
							<DateTimeInput bind:value={paused_until} />
						{/if}
					</div>
				</div>
			</Section>

			<Section label="Runnable">
				{#if !hideTarget}
					{#if fixedScriptPath != ''}
						<PipelineLockedRunnableInfo path={fixedScriptPath} />
					{:else if !edit}
						<p class="text-xs mb-1 text-secondary">
							Pick a script or flow to be triggered by the schedule<Required required={true} />
						</p>
						<ScriptPicker
							workspace={wsId}
							disabled={(initialScriptPath != '' && !initNewPath) || !can_write}
							initialPath={initialScriptPath}
							kinds={['script']}
							allowFlow={true}
							allowRefresh={can_write}
							bind:itemKind
							bind:scriptPath={script_path}
							clearable
						/>
					{:else}
						<Alert type="info" title="Runnable path cannot be edited" collapsible>
							Once a schedule is created, the runnable path cannot be changed. However, when
							renaming a script or a flow, the runnable path will automatically update itself.
						</Alert>
						<div class="my-2"></div>
						<ScriptPicker
							workspace={wsId}
							disabled
							initialPath={script_path}
							scriptPath={script_path}
							allowFlow={true}
							{itemKind}
							allowView={script_path != '' && !!runnable}
							allowEdit={script_path != '' && !!runnable && canEditRunnable}
						/>
					{/if}
					{#if itemKind == 'flow'}
						<Toggle
							options={{ right: 'no overlap of flows' }}
							bind:checked={no_flow_overlap}
							class="mt-2"
						/>
					{/if}
					{#if itemKind == 'script'}
						<div class="flex gap-2 items-center mt-2">
							<Toggle options={{ right: 'no overlap' }} checked={true} disabled /><Tooltip
								>Currently, overlapping scripts' executions is not supported. The next execution
								will be scheduled only after the previous iteration has completed.</Tooltip
							>
						</div>
					{/if}
				{/if}
				<div class={!hideTarget ? 'mt-6' : ''}>
					{#if !loading}
						{#if runnable || draftSchema}
							{@const schema = draftSchema ?? runnable?.schema}
							{#if schema && schema.properties && Object.keys(schema.properties).length > 0}
								{#await import('$lib/components/SchemaForm.svelte')}
									<Loader2 class="animate-spin" />
								{:then Module}
									<Module.default
										showReset
										onlyMaskPassword
										disabled={!can_write}
										schema={$state.snapshot(schema)}
										bind:isValid
										bind:args
									/>
								{/await}
							{:else}
								<div class="text-xs text-secondary">
									This {is_flow ? 'flow' : 'script'} takes no argument
								</div>
							{/if}
						{:else if script_path != ''}
							<div class="text-xs text-secondary my-2">
								You cannot see the the {is_flow ? 'flow' : 'script'} input form as you do not have access
								to it.
							</div>
						{:else}
							<div class="text-xs text-secondary my-2">
								Pick a {is_flow ? 'flow' : 'script'} and fill its argument here
							</div>
						{/if}
					{:else}
						<Loader2 class="animate-spin" />
					{/if}
				</div>
			</Section>

			<Section label="Advanced" collapsable>
				{#snippet header()}
					<TriggerAdvancedBadges
						error_handler_path={errorHandlerPath}
						{retry}
						extraBadges={[
							{ name: 'Recovery Handler', active: !!recoveryHandlerPath },
							{ name: 'Success Handler', active: !!successHandlerPath },
							{ name: 'Dynamic Skip', active: !!dynamicSkipPath },
							{ name: 'Custom Tag', active: !!tag }
						]}
					/>
				{/snippet}
				{@render errorHandler()}
			</Section>
			<div class="pb-8" />
		</div>
	{/if}
{/snippet}

{#snippet errorHandler()}
	<div class="flex flex-col gap-2 min-h-96">
		{#if !drawerLoading}
			<Tabs bind:selected={optionTabSelected}>
				<Tab value="error_handler" label="Error Handler" />
				<Tab value="recovery_handler" label="Recovery Handler" />
				<Tab value="success_handler" label="Success Handler" />
				<Tab value="retries" label="Retries" />
				<Tab value="dynamic_skip" label="Dynamic skip" />
				{#if itemKind === 'script'}
					<Tab value="tag" label="Custom tag" />
				{/if}
			</Tabs>
			{#if optionTabSelected === 'error_handler'}
				<Section label="Error handler">
					{#snippet header()}
						<div class="flex flex-row gap-2">
							{#if !$enterpriseLicense}<span class="text-xs text-secondary">(ee only)</span>{/if}
						</div>
					{/snippet}
					{#snippet action()}
						<div class="flex flex-row items-center gap-1 text-xs text-secondary">
							<Dropdown
								disabled={!can_write}
								items={[
									{
										displayName: `Override future schedules only`,
										action: () => saveAsDefaultErrorHandler(false)
									},
									{
										displayName: 'Override all existing',
										type: 'delete',
										action: () => saveAsDefaultErrorHandler(true)
									}
								]}
							>
								{#snippet buttonReplacement()}
									<Save size={12} class="mr-1" />
									Set as default
								{/snippet}
							</Dropdown>
						</div>
					{/snippet}
					<div class="flex flex-row py-2">
						<Toggle
							size="xs"
							disabled={!can_write || !$enterpriseLicense}
							bind:checked={wsErrorHandlerMuted}
							options={{ right: 'Mute workspace error handler for this schedule' }}
						/>
					</div>

					<ErrorOrRecoveryHandler
						workspace={wsId}
						isEditable={can_write}
						errorOrRecovery="error"
						showScriptHelpText={true}
						bind:handlerSelected={errorHandlerSelected}
						bind:handlerPath={errorHandlerPath}
						toggleText="Alert channel on error"
						customScriptTemplate="/scripts/add?hub=hub%2F19743%2Fwindmill%2Fschedule_error_handler_template"
						bind:customHandlerKind={errorHandleritemKind}
						bind:handlerExtraArgs={errorHandlerExtraArgs}
					>
						{#snippet customTabTooltip()}
							<Tooltip>
								<div class="flex gap-20 items-start mt-3">
									<div class="text-xs"
										>The following args will be passed to the error handler:
										<ul class="mt-1 ml-2">
											<li
												><b>workspace_id</b>: The ID of the workspace that the schedule belongs to.</li
											>
											<li><b>job_id</b>: The UUID of the job that errored.</li>
											<li><b>path</b>: The path of the script or flow that failed.</li>
											<li><b>is_flow</b>: Whether the runnable is a flow.</li>
											<li><b>schedule_path</b>: The path of the schedule.</li>
											<li><b>error</b>: The error details.</li>
											<li
												><b>failed_times</b>: Minimum number of times the schedule failed before
												calling the error handler.</li
											>
											<li><b>started_at</b>: The start datetime of the latest job that failed.</li>
										</ul>
									</div>
								</div>
							</Tooltip>
						{/snippet}
					</ErrorOrRecoveryHandler>
					<div class="flex flex-row items-center justify-between">
						<div class="flex flex-row items-center mt-4 font-semibold text-xs gap-2">
							<p class={emptyString(errorHandlerPath) ? 'text-primary' : ''}>
								Triggered when schedule failed</p
							>
							<select
								class="!w-14"
								bind:value={failedExact}
								disabled={!$enterpriseLicense || emptyString(errorHandlerPath)}
							>
								<option value={false}>&gt;=</option>
								<option value={true}>==</option>
							</select>
							<input
								type="number"
								class="!w-14 text-center {emptyString(errorHandlerPath) ? 'text-primary' : ''}"
								bind:value={failedTimes}
								disabled={!$enterpriseLicense}
								min="1"
							/>
							<p class={emptyString(errorHandlerPath) ? 'text-primary' : ''}
								>time{failedTimes > 1 ? 's in a row' : ''}</p
							>
						</div>
					</div>
				</Section>
			{:else if optionTabSelected === 'recovery_handler'}
				{@const disabled = !can_write || emptyString($enterpriseLicense)}
				<Section label="Recovery handler">
					{#snippet header()}
						<div class="flex flex-row gap-2">
							{#if !$enterpriseLicense}<span class="text-xs text-secondary">(ee only)</span>{/if}
						</div>
					{/snippet}
					{#snippet action()}
						<div class="flex flex-row items-center text-secondary text-xs gap-2">
							defaults
							<Dropdown
								{disabled}
								items={[
									{
										displayName: `Override future schedules only`,
										action: () => saveAsDefaultRecoveryHandler(false)
									},
									{
										displayName: 'Override all existing',
										type: 'delete',
										action: () => saveAsDefaultRecoveryHandler(true)
									}
								]}
							>
								{#snippet buttonReplacement()}
									<Save size={12} class="mr-1" />
									Set as default
								{/snippet}
							</Dropdown>
						</div>
					{/snippet}
					<ErrorOrRecoveryHandler
						workspace={wsId}
						isEditable={!disabled}
						errorOrRecovery="recovery"
						bind:handlerSelected={recoveryHandlerSelected}
						bind:handlerPath={recoveryHandlerPath}
						toggleText="Alert channel when error recovered"
						customScriptTemplate="/scripts/add?hub=hub%2F9082%2Fwindmill%2Fschedule_recovery_handler_template"
						bind:customHandlerKind={recoveryHandlerItemKind}
						bind:handlerExtraArgs={recoveryHandlerExtraArgs}
					>
						{#snippet customTabTooltip()}
							<Tooltip>
								<div class="flex gap-20 items-start mt-3">
									<div class="text-xs"
										>The following args will be passed to the recovery handler:
										<ul class="mt-1 ml-2">
											<li><b>path</b>: The path of the script or flow that recovered.</li>
											<li><b>is_flow</b>: Whether the runnable is a flow.</li>
											<li><b>schedule_path</b>: The path of the schedule.</li>
											<li><b>error</b>: The error of the last job that errored</li>
											<li
												><b>error_started_at</b>: The start datetime of the last job that errored</li
											>
											<li
												><b>success_times</b>: The number of times the schedule succeeded before
												calling the recovery handler.</li
											>
											<li><b>success_result</b>: The result of the latest successful job</li>
											<li
												><b>success_started_at</b>: The start datetime of the latest successful job</li
											>
										</ul>
									</div>
								</div>
							</Tooltip>
						{/snippet}
					</ErrorOrRecoveryHandler>
					<div class="flex flex-row items-center justify-between">
						<div
							class="flex flex-row items-center mt-5 font-semibold text-xs {emptyString(
								recoveryHandlerPath
							)
								? 'text-primary'
								: ''}"
						>
							<p>Triggered when schedule recovered</p>
							<input
								type="number"
								class="!w-14 mx-2 text-center"
								bind:value={recoveredTimes}
								min="1"
								{disabled}
							/>
							<p>time{recoveredTimes > 1 ? 's in a row' : ''}</p>
						</div>
					</div>
				</Section>
			{:else if optionTabSelected === 'success_handler'}
				{@const disabled = !can_write || emptyString($enterpriseLicense)}
				<Section label="Success handler">
					{#snippet header()}
						<div class="flex flex-row gap-2">
							{#if !$enterpriseLicense}<span class="text-xs text-secondary">(ee only)</span>{/if}
						</div>
					{/snippet}
					{#snippet action()}
						<div class="flex flex-row items-center text-secondary text-xs gap-2">
							defaults
							<Dropdown
								{disabled}
								items={[
									{
										displayName: `Override future schedules only`,
										action: () => saveAsDefaultSuccessHandler(false)
									},
									{
										displayName: 'Override all existing',
										type: 'delete',
										action: () => saveAsDefaultSuccessHandler(true)
									}
								]}
							>
								{#snippet buttonReplacement()}
									<Save size={12} class="mr-1" />
									Set as default
								{/snippet}
							</Dropdown>
						</div>
					{/snippet}
					<ErrorOrRecoveryHandler
						workspace={wsId}
						isEditable={!disabled}
						errorOrRecovery="success"
						bind:handlerSelected={successHandlerSelected}
						bind:handlerPath={successHandlerPath}
						toggleText="Alert channel when successful"
						customScriptTemplate="/scripts/add?hub=hub%2F9071%2Fwindmill%2Fschedule_success_handler_template"
						bind:customHandlerKind={successHandlerItemKind}
						bind:handlerExtraArgs={successHandlerExtraArgs}
					>
						{#snippet customTabTooltip()}
							<Tooltip>
								<div class="flex gap-20 items-start mt-3">
									<div class="text-xs"
										>The following args will be passed to the success handler:
										<ul class="mt-1 ml-2">
											<li><b>path</b>: The path of the script or flow that succeeded.</li>
											<li><b>is_flow</b>: Whether the runnable is a flow.</li>
											<li><b>schedule_path</b>: The path of the schedule.</li>
											<li><b>success_result</b>: The result of the successful job</li>
											<li><b>success_started_at</b>: The start datetime of the successful job</li>
										</ul>
									</div>
								</div>
							</Tooltip>
						{/snippet}
					</ErrorOrRecoveryHandler>
				</Section>
			{:else if optionTabSelected === 'retries'}
				{@const disabled = !can_write || emptyString($enterpriseLicense)}
				<Section label="Retries">
					{#snippet header()}
						<div class="flex flex-row gap-2">
							{#if !$enterpriseLicense}<span class="text-xs text-secondary">(ee only)</span>{/if}
						</div>
						<Tooltip>
							If defined, upon error this schedule will be retried with a delay and a maximum number
							of attempts as defined below.
							<br />
							This is only available for individual script. For flows, retries can be set on each flow
							step in the flow editor.
						</Tooltip>
					{/snippet}
					{#if itemKind !== 'script'}
						<Alert type="info" title="Only available for scripts" class="mb-2">
							Error Handler and Retries are only available for scripts. For flows, use the built-in <a
								href="https://www.windmill.dev/docs/flows/flow_error_handler"
								target="_blank">error handler</a
							>
							and <a href="https://www.windmill.dev/docs/flows/retries" target="_blank">retries</a>.
						</Alert>
					{:else}
						<FlowRetries
							bind:flowModuleRetry={retry}
							disabled={itemKind !== 'script' || disabled}
						/>
					{/if}
				</Section>
			{:else if optionTabSelected === 'dynamic_skip'}
				<Section label="Dynamic skip">
					{#snippet header()}
						<Tooltip>
							Optional script to filter scheduled dates. Receives the proposed datetime and returns
							boolean. True = run on this date, False = skip to next occurrence.
						</Tooltip>
					{/snippet}
					<div class="flex flex-col gap-6">
						<Label label="Dynamic skip script">
							<div class="flex flex-row">
								<ScriptPicker
									workspace={wsId}
									disabled={!can_write}
									bind:scriptPath={dynamicSkipPath}
									kinds={['script']}
									allowRefresh={can_write}
									clearable
								/>
								{#if !dynamicSkipPath}
									<Button
										btnClasses="ml-4 whitespace-nowrap"
										variant="default"
										size="xs"
										href="/scripts/add?hub=hub%2F19822%2Fwindmill%2Fdynamic_skip_template{wsParam}"
										disabled={!can_write}
										target="_blank"
									>
										Create from template
									</Button>
								{/if}
							</div>
						</Label>
						<Alert type="info" size="xs" title="Handler requirements">
							Handler must return a boolean value. Return true to execute the scheduled job, false
							to skip.
						</Alert>
					</div>
				</Section>
			{:else if optionTabSelected === 'tag'}
				<Section
					label="Custom script tag"
					tooltip="When set, the script tag will be overridden by this tag"
				>
					<WorkerTagPicker
						bind:tag
						workspaceId={wsId}
						popupPlacement="top-end"
						disabled={!can_write}
					/>
				</Section>
			{/if}
		{:else}
			<Loader2 class="animate-spin" />
		{/if}
	</div>
{/snippet}

{#if useDrawer}
	<Drawer size="900px" bind:this={drawer} on:close={() => clearPageDrawerAnchor(SCHEDULES_PATH)}>
		<DrawerContent
			bannerReserved={hasBaseline}
			title={edit
				? can_write
					? `Edit schedule ${schedulePath}`
					: `View schedule ${schedulePath}`
				: 'New schedule'}
			on:close={drawer.closeDrawer}
		>
			{#snippet actions()}
				<div class="flex flex-row gap-4 items-center">
					{@render saveButton()}
				</div>
			{/snippet}
			{#snippet banner()}
				<LocalDraftBanner
					show={hasBaseline && item.dirty}
					getDeployed={() => item.deployed}
					reserveSpace={hasBaseline}
					getCurrent={() => item.value}
					onDiscard={async () => void (await item.discard())}
					disabled={!can_write}
				/>
			{/snippet}
			{@render content()}
		</DrawerContent>
	</Drawer>
{:else}
	<Section label={!customLabel ? 'Schedule' : ''} headerClass="grow min-w-0 h-[30px]">
		{#snippet header()}
			{#if customLabel}
				{@render customLabel()}
			{/if}
		{/snippet}
		{#snippet action()}
			<div class="flex flex-row gap-2 items-center">
				{@render saveButton()}
			</div>
		{/snippet}
		{#if docDescription}
			{@render docDescription()}
		{/if}
		{@render content()}
	</Section>
{/if}
