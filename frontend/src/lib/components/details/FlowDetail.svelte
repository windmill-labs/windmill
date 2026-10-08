<script lang="ts">
	import {
		FlowService,
		JobService,
		WorkspaceService,
		type Flow,
		type FlowModule,
		type TriggersCount,
		type WorkspaceDeployUISettings
	} from '$lib/gen'
	import {
		canWrite,
		defaultIfEmptyString,
		emptyString,
		urlParamsToObject,
		extractTagFromSharableHash,
		interpolateTag,
		isDynamicTag,
		isTagTemplate
	} from '$lib/utils'
	import { isDeployable, ALL_DEPLOYABLE } from '$lib/utils_deployable'

	import DetailPageLayout from '$lib/components/details/DetailPageLayout.svelte'
	import OnBehalfOfBadge from '$lib/components/details/OnBehalfOfBadge.svelte'
	import { goto } from '$lib/navigation'
	import { base } from '$lib/base'
	import { Badge as HeaderBadge, Alert } from '$lib/components/common'
	import MoveDrawer from '$lib/components/MoveDrawer.svelte'
	import RunForm from '$lib/components/RunForm.svelte'
	import type { PendingRun } from '$lib/components/details/pendingRun'
	import { processSecretArgs } from '$lib/components/secretArgUtils'
	import type { Schema } from '$lib/common'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import { enterpriseLicense, userStore, userWorkspaces, workspaceStore } from '$lib/stores'
	import { useOperatorBuilderFlows } from '$lib/operatorWriteRights'
	import { sendUserToast } from '$lib/toast'
	import DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import SavedInputsV2 from '$lib/components/SavedInputsV2.svelte'
	import AIFormAssistant from '$lib/components/copilot/AIFormAssistant.svelte'
	import {
		FolderOpen,
		Archive,
		Trash,
		ChevronUpSquare,
		Shield,
		Loader2,
		GitFork,
		Play,
		History,
		Pen,
		Eye,
		HistoryIcon,
		LayoutDashboard
	} from 'lucide-svelte'

	import DetailPageHeader from '$lib/components/details/DetailPageHeader.svelte'
	import FlowGraphViewer from '$lib/components/FlowGraphViewer.svelte'
	import { createRawAppFromFlow } from '$lib/components/details/createRawAppFromScript'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import FlowGraphViewerStep from '$lib/components/FlowGraphViewerStep.svelte'
	import GfmMarkdown from '$lib/components/GfmMarkdown.svelte'
	import FlowHistory from '$lib/components/flows/FlowHistory.svelte'
	import Star from '$lib/components/Star.svelte'

	import { writable } from 'svelte/store'
	import InputSelectedBadge from '$lib/components/schema/InputSelectedBadge.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { onDestroy, tick, untrack } from 'svelte'
	import LogViewer from '$lib/components/LogViewer.svelte'
	import TriggersEditor from '$lib/components/triggers/TriggersEditor.svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import { setContext } from 'svelte'
	import TriggersBadge from '$lib/components/graph/renderers/triggers/TriggersBadge.svelte'
	import { Triggers } from '$lib/components/triggers/triggers.svelte'
	import FlowAssetsHandler, {
		initFlowGraphAssetsCtx
	} from '$lib/components/flows/FlowAssetsHandler.svelte'
	import { interceptNav } from '$lib/components/details/interceptNav'
	import {
		setOperatingWorkspace,
		useOperatingUser,
		useOperatingWorkspaceHref
	} from '$lib/components/operatingWorkspace.svelte'
	import FlowChat from '$lib/components/flows/conversations/FlowChat.svelte'
	import { slide } from 'svelte/transition'
	import { twMerge } from 'tailwind-merge'
	import CiTestResults from '$lib/components/CiTestResults.svelte'
	import NoDirectDeployAlert from '$lib/components/NoDirectDeployAlert.svelte'
	import {
		buildForkEditUrl,
		editInForkAllowed,
		editInForkDescription,
		editInForkLabel,
		onEditInForkClick,
		openEditInFork
	} from '$lib/utils/editInFork'
	import { isCloudHosted } from '$lib/cloud'

	let {
		path: routePath,
		workspace,
		searchParams,
		locationHash = '',
		onNavigate = goto,
		active = true,
		embedded = false,
		pendingRun,
		onLoadState
	}: {
		/** The `[...path]` route segment: the flow's path. */
		path: string
		workspace: string | undefined
		/** Query of the URL this detail was opened at (`version`). */
		searchParams?: URLSearchParams
		/** `#`-fragment of that URL, carrying prefilled run args from `Run again`. */
		locationHash?: string
		onNavigate?: (url: string) => void | Promise<void>
		/** Gates the window-level Ctrl+Enter handler, so a hidden instance cannot steal it. */
		active?: boolean
		/** Rendered inside a page that is not the flow's own (an AI session preview tab), whose
		 * URL this must leave alone and which a cross-workspace link must not navigate away. */
		embedded?: boolean
		/** A chat tool call waiting on this form, when the reader chose to confirm it here
		 * rather than on the card. Seeds the arguments it proposed and takes over Run; see
		 * {@link PendingRun} for why the page must not run it itself. */
		pendingRun?: PendingRun
		/** How the load ended, for a host that renders its own state around this page. */
		onLoadState?: (state: 'loaded' | 'not_found') => void
	} = $props()

	// Every workspace-scoped call below and in the drawers this page opens targets
	// `workspace`, not the nav store: a session views a flow in its own workspace.
	setOperatingWorkspace(() => workspace)

	// Roles are per-workspace, so every gate below asks about `workspace` rather than about
	// the one the browser is navigated to. Unresolved reads as unknown, which `canWrite`
	// refuses — the safe answer while a fork's `whoami` is still in flight.
	const operatingUser = useOperatingUser()
	// A ⌘-click opens these in a new tab, which only the query names the workspace for.
	const operatingHref = useOperatingWorkspaceHref()
	const actingUser = $derived(operatingUser.current)
	const operatorBuilderFlows = useOperatorBuilderFlows()

	let flow: Flow | undefined = $state()
	// Derived, not assigned during the load: in a fork the acting user is looked up
	// asynchronously, and a value read after `getFlowByPath` resolves can land before that
	// lookup does. Assigned once, that leaves the page permanently read-only — `canWrite`
	// refuses an unknown user — with nothing to recompute it.
	const can_write = $derived(!!flow && canWrite(flow.path, flow.extra_perms!, actingUser))
	const promptForAi = $derived((flow?.schema?.prompt_for_ai as string | undefined) ?? '')
	let shareModal: ShareModal | undefined = $state()

	let scheduledForStr: string | undefined = $state(undefined)
	let invisible_to_owner: boolean | undefined = $state(undefined)
	let overrideTag: string | undefined = $state(undefined)
	let overrideTagNote: string | undefined = $state(undefined)
	// Tag carried over from 'Run again', pending the dynamic-tag check in loadFlow
	let carriedTag: string | undefined = undefined
	let inputSelected: 'saved' | 'history' | undefined = $state(undefined)
	let jsonView = $state(false)
	let deploymentInProgress = $state(false)
	let deploymentJobId: string | undefined = $state(undefined)

	let intervalId: number | undefined = undefined

	const triggersCount = writable<TriggersCount | undefined>(undefined)

	// Add triggers context store
	const triggersState = $state(
		new Triggers([
			{ type: 'webhook', path: '', isDraft: false },
			{ type: 'default_email', path: '', isDraft: false },
			{ type: 'cli', path: '', isDraft: false }
		])
	)
	setContext<TriggerContext>('TriggerContext', {
		triggersCount,
		simplifiedPoll: writable(false),
		showCaptureHint: writable(undefined),
		triggersState
	})

	setContext(
		'FlowGraphAssetContext',
		initFlowGraphAssetsCtx({ getModules: () => flow?.value.modules ?? [] })
	)

	// `${path}|${version}` so navigating between pinned and latest re-runs loadFlow.
	let previousLoadKey: string | undefined = $state(undefined)

	async function archiveFlow(): Promise<void> {
		await FlowService.archiveFlowByPath({
			workspace: workspace!,
			path,
			requestBody: { archived: !flow?.archived }
		})
		loadFlow()
	}

	async function deleteFlow(): Promise<void> {
		await FlowService.deleteFlowByPath({ workspace: workspace!, path })
		sendUserToast('Flow deleted')
		// Embedded, the host owns the tab: reloading lets it show that nothing is deployed
		// here, rather than opening Home beside a tab still showing the deleted flow.
		if (embedded) {
			loadFlow().then(
				() => onLoadState?.('loaded'),
				(e) => e?.status === 404 && onLoadState?.('not_found')
			)
		} else onNavigate('/')
	}

	async function loadTriggersCount() {
		$triggersCount = await FlowService.getTriggersCountOfFlow({
			workspace: workspace!,
			path
		})
	}

	async function loadTriggers(): Promise<void> {
		await triggersState.fetchTriggers(triggersCount, workspace, path, true, undefined, actingUser)
	}

	let pinnedVersion: number | undefined = $state(undefined)

	async function loadFlow(): Promise<void> {
		const versionParam = searchParams?.get('version')
		const versionId = versionParam ? Number(versionParam) : NaN
		if (Number.isFinite(versionId)) {
			const versioned = await FlowService.getFlowVersion({
				workspace: workspace!,
				version: versionId
			})
			if (versioned.path !== path) {
				sendUserToast(`Flow version ${versionId} belongs to ${versioned.path}, not ${path}.`, true)
				onNavigate(`/flows/get/${versioned.path}?workspace=${workspace}&version=${versionId}`)
				return
			}
			flow = versioned
			pinnedVersion = versionId
		} else {
			flow = await FlowService.getFlowByPath({
				workspace: workspace!,
				path,
				withStarredInfo: true
			})
			pinnedVersion = undefined
		}
		// A carried non-template value equal to the resolution of the flow's own dynamic
		// tag for these args is not an override but the previous run's pinned resolution:
		// drop it so the backend re-resolves from the (possibly edited) args. A differing
		// value is a genuine user override and a template re-resolves at push, so both stay.
		if (
			carriedTag &&
			isDynamicTag(flow.tag) &&
			!isTagTemplate(carriedTag) &&
			interpolateTag(flow.tag ?? '', workspace!, args) === carriedTag
		) {
			if (overrideTag === carriedTag) {
				overrideTag = undefined
				overrideTagNote = `tag ${flow.tag} is resolved at run time, so the previous run's tag ${carriedTag} was not applied`
			}
			carriedTag = undefined
		}
		if (!flow.path.startsWith(`u/${actingUser?.username}`) && flow.path.split('/').length > 2) {
			invisible_to_owner = flow.visible_to_runner_only
		}
		intervalId && clearInterval(intervalId)
		deploymentInProgress = flow.lock_error_logs == ''
		if (deploymentInProgress) {
			intervalId = setInterval(syncer, 500)
		}
	}

	let isValid = $state(true)
	let loading = $state(false)

	async function syncer(): Promise<void> {
		if (flow) {
			const status = await FlowService.getFlowDeploymentStatus({
				workspace: workspace!,
				path: flow.path
			})
			if (status.lock_error_logs == undefined || status.lock_error_logs != '') {
				deploymentInProgress = false
				deploymentJobId = undefined
				flow.lock_error_logs = status.lock_error_logs
				clearInterval(intervalId)
			} else if (status.job_id) {
				deploymentJobId = status.job_id
			}
		}
	}

	async function runFlow(
		scheduledForStr: string | undefined,
		args: Record<string, any>,
		invisibleToOwner: boolean | undefined,
		overrideTag: string | undefined
	) {
		loading = true
		const scheduledFor = scheduledForStr ? new Date(scheduledForStr).toISOString() : undefined
		try {
			let run = await JobService.runFlowByPath({
				workspace: workspace!,
				path,
				invisibleToOwner,
				requestBody: args,
				scheduledFor,
				tag: overrideTag,
				skipPreprocessor: true
			})
			await onNavigate('/run/' + run + '?workspace=' + workspace)
		} catch (e) {
			throw e
		} finally {
			loading = false
		}
	}

	async function runFlowForChat(
		userMessage: string,
		conversationId: string,
		additionalInputs?: Record<string, any>
	): Promise<string> {
		// A chat flow's inputs reach the job straight from the composer, with no RunForm in
		// between to mint a secret as it is typed — so this is the only place a value the
		// schema marks `password` can become a reference. Without it the literal is stored in
		// the job's arguments, where anyone who can read the run can read it. Idempotent, so a
		// reference that was already minted costs a walk and no round trip.
		const requestBody = await processSecretArgs(
			{ user_message: userMessage, ...(additionalInputs ?? {}) },
			flow?.schema as Schema | undefined,
			workspace
		)
		const run = await JobService.runFlowByPath({
			workspace: workspace!,
			path,
			memoryId: conversationId,
			requestBody,
			skipPreprocessor: true
		})
		return run
	}

	// While a chat call is carried here, the form edits that call's own draft — the one the
	// card edits — rather than a copy of it. There is then nothing to keep in step and nothing
	// to carry back: whatever is typed on either surface is what the call runs with.
	let ownArgs: Record<string, any> | undefined = $state(undefined)
	// Falls back once the draft is gone: pressing Run settles the call, and the form must go
	// on showing what it submitted while the job starts — and still show it if the start is
	// refused, which leaves the reader on the page with the run to make again.
	const args = $derived(pendingRun?.draftArgs ?? ownArgs)
	function setArgs(next: Record<string, any> | undefined) {
		if (pendingRun) pendingRun.setDraftArgs(next ?? {})
		else ownArgs = next
	}

	// Read once on purpose: these args seed the form, so tracking the fragment would
	// overwrite what the user has typed whenever it changes.
	let hash = untrack(() => locationHash)
	if (hash.length > 1) {
		try {
			let searchParams = new URLSearchParams(hash.slice(1))
			carriedTag = extractTagFromSharableHash(searchParams)
			overrideTag = carriedTag
			let params = [...Object.entries(urlParamsToObject(searchParams))].map(([k, v]) => [
				k,
				JSON.parse(v)
			])
			setArgs(Object.fromEntries(params))
		} catch (e) {
			console.error('Was not able to transform hash as args', e)
		}
	}

	// Operators with the builder right author flows out of deployed runnables; every other
	// operator is read-only here.
	let canAuthorFlow = $derived(!actingUser?.operator || $operatorBuilderFlows)

	let moveDrawer: MoveDrawer | undefined = $state()
	let deploymentDrawer: DeployWorkspaceDrawer | undefined = $state()
	let runForm: RunForm | undefined = $state()

	// Run hands the arguments to the waiting call instead of starting a job: the tool that
	// parked on this form starts one itself when it resumes.
	//
	// Bound to the call it was built for, not to whichever one the page is carrying when the
	// press lands: a press runs across `processSecretArgs`, and the call can settle and be
	// replaced by the next request for this same item inside that round trip. Submitting to
	// whatever is current would start that one without its reader ever confirming it.
	const runAction = $derived.by(() => {
		const call = pendingRun
		if (!call) return runFlow
		return (_scheduledForStr: string | undefined, a: Record<string, any>) => {
			// Kept for the moment the draft stops answering, just below.
			ownArgs = a
			if (!call.submit(a)) {
				sendUserToast('That request is no longer waiting on this form', true)
			}
		}
	})

	// The dev workspace's editor is not one the session panel can host, so from a preview tab
	// it opens in a new browser tab, as the session editors' own entry does.
	function editInFork(e: Event | undefined, itemPath: string) {
		if (!embedded) {
			return onEditInForkClick(e, 'flow', itemPath, { hasHref: true, prodWorkspace: workspace })
		}
		const m = e as MouseEvent | undefined
		if (m && (m.metaKey || m.ctrlKey || m.shiftKey || m.altKey || (m.button ?? 0) !== 0)) return
		e?.preventDefault()
		return openEditInFork('flow', itemPath, workspace)
	}

	function getMainButtons(flow: Flow | undefined, args: object | undefined) {
		const buttons: any = []

		if (flow && canAuthorFlow) {
			buttons.push({
				label: 'Fork',
				description: `Start a new flow from a copy of this one`,
				narrow: { dropdownOf: 'Edit' },
				buttonProps: {
					href: operatingHref(`${base}/flows/add?template=${flow.path}`),
					onClick: interceptNav(onNavigate, `/flows/add?template=${flow.path}`),
					variant: 'subtle',
					unifiedSize: 'md',
					disabled: !showEditButtons,
					startIcon: GitFork
				}
			})
		}

		if (
			flow &&
			!actingUser?.operator &&
			!isCloudHosted() &&
			editInForkAllowed(workspace, $userWorkspaces)
		) {
			buttons.push({
				label: editInForkLabel(workspace, $userWorkspaces),
				description: editInForkDescription('flow', workspace, $userWorkspaces),
				narrow: { dropdownOf: 'Edit' },
				buttonProps: {
					href: buildForkEditUrl('flow', flow.path, workspace),
					onClick: (e: Event | undefined) => editInFork(e, flow.path),
					unifiedSize: 'md',
					variant: !showEditButtons ? 'default' : 'subtle',
					startIcon: Pen
				}
			})
		}

		if (!flow) {
			return buttons
		}

		buttons.push({
			label: `Runs`,
			buttonProps: {
				href: operatingHref(`${base}/runs/${flow.path}`),
				onClick: interceptNav(onNavigate, `/runs/${flow.path}`),
				unifiedSize: 'md',
				variant: 'subtle',
				startIcon: Play
			}
		})

		buttons.push({
			label: `History`,
			narrow: 'menu',
			buttonProps: {
				onClick: () => flowHistory?.open(),
				unifiedSize: 'md',
				variant: 'subtle',
				startIcon: History
			}
		})

		if (!flow || !canAuthorFlow || !can_write) {
			return buttons
		}

		// The builder right covers flows only; building an app is still refused to operators.
		if (!actingUser?.operator) {
			buttons.push({
				label: 'Build app',
				narrow: 'menu',
				buttonProps: {
					onClick: async () => {
						const app = createRawAppFromFlow(flow.path, flow.summary, flow.schema)
						// The raw app editor reads the payload from sessionStorage on mount.
						sessionStorage.setItem('rawAppImport', JSON.stringify(app))
						await onNavigate('/apps_raw/add')
					},
					unifiedSize: 'md',
					variant: 'subtle',
					disabled: !showEditButtons,
					startIcon: LayoutDashboard
				}
			})
		}

		buttons.push({
			label: 'Edit',
			buttonProps: {
				href: operatingHref(`${base}/flows/edit/${path}`),
				onClick: interceptNav(onNavigate, `/flows/edit/${path}`),
				variant: 'accent',
				unifiedSize: 'md',
				disabled: !can_write || !showEditButtons,
				startIcon: Pen
			}
		})
		return buttons
	}

	let deployUiSettings: WorkspaceDeployUISettings | undefined = $state(undefined)

	async function getDeployUiSettings() {
		if (!$enterpriseLicense) {
			deployUiSettings = ALL_DEPLOYABLE
			return
		}
		let settings = await WorkspaceService.getPublicSettings({ workspace: workspace! })
		deployUiSettings = settings.deploy_ui ?? ALL_DEPLOYABLE
	}
	getDeployUiSettings()

	function getMenuItems(
		flow: Flow | undefined,
		deployUiSettings: WorkspaceDeployUISettings | undefined
	) {
		if (!flow || !canAuthorFlow) return []

		const menuItems: any = []

		menuItems.push({
			label: 'Permissions',
			onclick: () => shareModal?.openDrawer(flow?.path ?? '', 'flow'),
			Icon: Shield,
			disabled: !can_write
		})

		if (showEditButtons) {
			menuItems.push({
				label: 'Move/Rename',
				onclick: () => moveDrawer?.openDrawer(flow?.path ?? '', flow?.summary, 'flow'),
				Icon: FolderOpen
			})
		}

		// The builder right opens this menu to operators; audit logs stay behind their own setting.
		if (
			!actingUser?.operator ||
			$userWorkspaces.find((w) => w.id === workspace)?.operator_settings?.audit_logs
		) {
			menuItems.push({
				label: 'Audit logs',
				Icon: Eye,
				onclick: () => {
					onNavigate(`/audit_logs?resource=${flow?.path}`)
				}
			})
		}

		if (isDeployable('flow', flow?.path ?? '', deployUiSettings) && !actingUser?.operator) {
			menuItems.push({
				label: 'Deploy to staging/prod',
				onclick: () => deploymentDrawer?.openDrawer(flow?.path ?? '', 'flow'),
				Icon: ChevronUpSquare
			})
		}

		if (can_write && showEditButtons) {
			menuItems.push({
				label: 'Deployments',
				onclick: () => flowHistory?.open(),
				Icon: HistoryIcon
			})
			menuItems.push({
				label: flow.archived ? 'Unarchive' : 'Archive',
				onclick: () => flow?.path && archiveFlow(),
				Icon: Archive,
				color: 'red'
			})
			menuItems.push({
				label: 'Delete',
				onclick: () => flow?.path && deleteFlow(),
				Icon: Trash,
				color: 'red'
			})
		}
		return menuItems
	}

	onDestroy(() => {
		intervalId && clearInterval(intervalId)
	})

	function onKeyDown(event: KeyboardEvent) {
		// Several instances can be mounted at once (a session keeps hidden tabs
		// mounted), and each one listens on the window: only the visible one runs.
		// `defaultPrevented` keeps anything nearer that already claimed the key — the chat
		// composer's send, a drawer's Escape — from also reaching the form.
		if (!active || event.defaultPrevented) return
		switch (event.key) {
			case 'Enter':
				if (event.ctrlKey || event.metaKey) {
					if (isValid) {
						event.preventDefault()
						runForm?.run()
					} else {
						sendUserToast('Please fix errors before running', true)
					}
				}
				break
		}
	}
	let stepDetail: FlowModule | string | undefined = $state(undefined)
	let rightPaneSelected = $state('saved_inputs')
	let savedInputsV2: SavedInputsV2 | undefined = $state(undefined)
	let detailLayout: DetailPageLayout | undefined = $state(undefined)
	let flowHistory: FlowHistory | undefined = $state(undefined)
	let path = $derived(routePath ?? '')

	let topSectionHeight = $state(0)
	let paneHeight = $state(0)
	let flowGraphHeight = $state(0)

	let graphMinHeight = $derived.by(() => {
		if (!topSectionHeight || !paneHeight) return 400
		const availableHeight = paneHeight - topSectionHeight - 1 // Account for the separator between the top section and the graph
		return Math.max(400, availableHeight)
	})

	$effect(() => {
		const cliTrigger = triggersState.triggers.find((t) => t.type === 'cli')
		if (cliTrigger) {
			cliTrigger.extra = {
				cliCommand: `wmill flow run ${flow?.path} -d '${JSON.stringify(args)}'`
			}
		}
	})
	$effect(() => {
		// `$userStore`, not the acting user: this asks whether the session is authenticated at
		// all, which is global. Waiting on the acting user instead would hold the load for a
		// fork's `whoami`, and never load it at all if that lookup failed.
		if (workspace && $userStore && routePath) {
			const versionParam = searchParams?.get('version') ?? ''
			const loadKey = `${workspace}|${path}|${versionParam}`
			if (previousLoadKey !== loadKey) {
				previousLoadKey = loadKey
				untrack(() => {
					// A path with no deployed flow rejects here, which is the normal state of a
					// draft the user has not deployed — reported so a host can say so in place
					// rather than leaving the page blank on an unhandled rejection.
					loadFlow().then(
						() => onLoadState?.('loaded'),
						(e) => {
							if (onLoadState && e?.status === 404) onLoadState('not_found')
							else sendUserToast('Could not load flow: ' + (e?.body ?? e?.message ?? e), true)
						}
					)
					loadTriggersCount()
					loadTriggers()
				})
			}
		}
	})
	let showEditButtons = $state(false)
	let mainButtons = $derived(getMainButtons(flow, args))
	let chatInputEnabled = $derived(flow?.value?.chat_input_enabled ?? false)
</script>

<svelte:window onkeydown={onKeyDown} />
<DeployWorkspaceDrawer bind:this={deploymentDrawer} />
<ShareModal bind:this={shareModal} />
<MoveDrawer
	bind:this={moveDrawer}
	on:update={async (e) => {
		await onNavigate('/flows/get/' + e.detail + `?workspace=${workspace}`)
		loadFlow()
	}}
/>
{#if flow}
	<FlowHistory bind:this={flowHistory} path={flow.path} onHistoryRestore={loadFlow} />
{/if}

<DetailPageLayout
	bind:this={detailLayout}
	bind:selected={rightPaneSelected}
	isOperator={actingUser?.operator}
	forceSmallScreen={chatInputEnabled}
	isChatMode={chatInputEnabled}
	flow_json={{
		value: flow?.value,
		summary: flow?.summary,
		description: flow?.description,
		schema: flow?.schema
	}}
>
	{#snippet header({ wide }: { wide: boolean })}
		<DetailPageHeader
			{wide}
			ownsPageHeader={!embedded}
			on:seeTriggers={() => {
				detailLayout?.showTriggers()
			}}
			{mainButtons}
			menuItems={getMenuItems(flow, deployUiSettings)}
			bind:errorHandlerMuted={
				() => flow?.ws_error_handler_muted ?? false,
				(v) => {
					if (flow !== undefined) flow.ws_error_handler_muted = v
				}
			}
			scriptOrFlowPath={flow?.path ?? ''}
			errorHandlerKind="flow"
			tag={flow?.tag ?? ''}
			labels={flow?.labels}
			inheritedLabels={flow?.inherited_labels}
			summary={flow?.summary}
			path={flow?.path}
			onSaved={can_write
				? async (newPath) => {
						if (newPath !== flow?.path) {
							await onNavigate(`/flows/get/${newPath}?workspace=${workspace}`)
						} else {
							loadFlow()
						}
					}
				: undefined}
		>
			{#snippet trigger_badges()}
				<TriggersBadge
					showOnlyWithCount={true}
					showDraft={false}
					{path}
					newItem={false}
					isFlow
					selected={rightPaneSelected == 'triggers'}
					onSelect={async (triggerIndex: number) => {
						detailLayout?.showTriggers()
						await tick()
						triggersState.selectedTriggerIndex = triggerIndex
					}}
					small={false}
				/>
			{/snippet}
			{#if workspace && flow}
				<!-- Favorites are the sidebar's, which lists the navigation workspace only. -->
				{#if workspace === $workspaceStore}
					<Star kind="flow" path={flow.path} summary={flow.summary} />
				{/if}
			{/if}
			<OnBehalfOfBadge
				onBehalfOf={flow?.on_behalf_of}
				onBehalfOfEmail={flow?.on_behalf_of_email}
				kind="flow"
			/>
			{#if flow?.value?.priority != undefined}
				<div class="hidden md:block">
					<HeaderBadge color="blue" variant="outlined" size="xs">
						{`Priority: ${flow?.value?.priority}`}
					</HeaderBadge>
				</div>
			{/if}
			{#if flow?.value?.concurrent_limit != undefined && flow?.value?.concurrency_time_window_s != undefined}
				<div class="hidden md:block">
					<HeaderBadge color="gray" variant="outlined" size="xs">
						{`Concurrency limit: ${flow?.value?.concurrent_limit} runs every ${flow?.value?.concurrency_time_window_s}s`}
					</HeaderBadge>
				</div>
			{/if}
		</DetailPageHeader>
	{/snippet}
	{#snippet form({ graphInline }: { graphInline: boolean })}
		<div class="px-3">
			<NoDirectDeployAlert onUpdateCanEditStatus={(v) => (showEditButtons = v)} />
		</div>
		{#if flow}
			<div class="flex flex-col h-full bg-surface divide-y" bind:clientHeight={paneHeight}>
				<div bind:clientHeight={topSectionHeight} class={twMerge(chatInputEnabled ? 'h-full' : '')}>
					<div
						class={twMerge(
							'w-full flex flex-col',
							chatInputEnabled ? 'h-full min-h-0' : 'max-w-3xl p-6 min-h-[300px] justify-center',
							'mx-auto'
						)}
					>
						<!-- The chat reaches the edges of the pane, so the notices above it carry their
						     own padding. `contents` leaves the form layout exactly as it was. -->
						<!-- Top spacing hangs off the first notice, not the wrapper: `{#if}` leaves a
						     comment anchor behind, so an empty wrapper is not `:empty` and its own
						     padding would show as a gap above a chat with nothing to announce. -->
						<div
							class={chatInputEnabled ? 'flex flex-col px-3 [&>*:first-child]:mt-3' : 'contents'}
						>
							{#if flow?.path}
								<CiTestResults path={flow.path} kind="flow" />
							{/if}

							{#if flow?.archived}
								<Alert type="error" title="Archived">This flow was archived</Alert>
								<div class="h-4"></div>
							{/if}

							{#if pinnedVersion !== undefined}
								<Alert type="info" title="Viewing pinned version {pinnedVersion}">
									This is a historical version of the flow, not the latest.
									<a
										class="underline"
										href="/flows/get/{path}?workspace={workspace}"
										onclick={interceptNav(onNavigate, `/flows/get/${path}`)}
									>
										View latest
									</a>
								</Alert>
								<div class="h-4"></div>
							{/if}

							<!-- In chat mode the description belongs to the chat, which shows it under the
							     empty transcript. -->
							{#if !chatInputEnabled && !emptyString(flow?.description)}
								<div class="p-4 rounded-md bg-surface-secondary">
									<GfmMarkdown
										md={defaultIfEmptyString(flow?.description, 'No description')}
										noPadding
									/>
								</div>
								<div class="h-4"></div>
							{/if}

							{#if deploymentInProgress}
								<div class="pb-4" transition:slide={{ duration: 150 }}>
									<HeaderBadge color="yellow">
										<Loader2 size={12} class="inline animate-spin mr-1" />
										Deployment in progress
										{#if deploymentJobId}
											<a
												href="/run/{deploymentJobId}?workspace={workspace}"
												class="underline"
												target="_blank">view job</a
											>
										{/if}
									</HeaderBadge>
								</div>
							{/if}
							{#if flow.lock_error_logs && flow.lock_error_logs != ''}
								<Alert type="error" title="Deployment failed">
									<p>
										This flow has not been deployed successfully because of the following errors:
									</p>
									<LogViewer content={flow.lock_error_logs} isLoading={false} tag={undefined} />
								</Alert>
								<div class="h-4"></div>
							{/if}
						</div>

						{#if chatInputEnabled}
							<!-- Chat Layout with Sidebar -->
							<FlowChat
								onRunFlow={runFlowForChat}
								{deploymentInProgress}
								path={flow?.path ?? ''}
								description={flow?.description}
								inputSchema={flow?.schema}
								flowModules={flow?.value?.modules}
								wideLayout
								frame="none"
							/>
						{:else}
							{@const hasSchema =
								flow.schema && Object.keys(flow.schema.properties ?? {}).length > 0}
							<!-- Normal Mode: Form Layout -->
							<div class="flex flex-col align-left">
								{#if hasSchema || inputSelected}
									<div
										class="flex flex-row justify-between min-h-12"
										transition:slide={{ duration: 150 }}
									>
										<InputSelectedBadge
											onReject={() => {
												savedInputsV2?.resetSelected()
											}}
											{inputSelected}
										/>

										{#if hasSchema && !pendingRun?.planModeActive}
											<Toggle
												bind:checked={jsonView}
												size="xs"
												options={{
													right: 'JSON',
													rightTooltip: 'Fill args from JSON'
												}}
												lightMode
											/>
										{/if}
									</div>
								{/if}

								{#snippet aiAssistant()}
									{#if Object.keys(flow?.schema?.properties ?? {}).length > 0}
										<AIFormAssistant
											instructions={promptForAi}
											onEditInstructions={can_write && canAuthorFlow
												? () => onNavigate(`/flows/edit/${flow?.path}`)
												: undefined}
											runnableType="flow"
											path={flow?.path}
										/>
									{/if}
								{/snippet}
								{#if promptForAi}
									{@render aiAssistant()}
								{/if}

								{#if pendingRun}
									<InputSelectedBadge
										inputSelected="pending_run"
										onReject={() => pendingRun.decline()}
									/>
								{/if}

								<!-- Keyed on the call, so a request arriving on a tab that is already open
								     builds a new form already holding what it proposed. Re-seeding one
								     that is already on screen would have to wait for it to exist. -->
								{#key pendingRun?.toolCallId}
									<RunForm
										bind:scheduledForStr
										bind:invisible_to_owner
										bind:overrideTag
										{overrideTagNote}
										syncArgsToUrl={!embedded}
										viewKeybinding
										{loading}
										autofocus
										detailed={false}
										bind:isValid
										runnable={flow}
										{runAction}
										claimRun={pendingRun}
										argsReadonly={pendingRun?.planModeActive}
										bind:args={() => args, setArgs}
										schedulable={!pendingRun}
										commonParams={!pendingRun}
										bind:this={runForm}
										{jsonView}
										actions={promptForAi ? undefined : aiAssistant}
									/>
								{/key}
							</div>

							<div class="pt-4 flex flex-col gap-1 w-full items-end">
								<span class="text-2xs text-secondary">
									Edited <TimeAgo date={flow.edited_at ?? ''} noSeconds /> by {flow.edited_by}
								</span>
							</div>
						{/if}
					</div>
				</div>
				{#if graphInline}
					<div class="grow min-h-0">
						<FlowGraphViewer
							triggerNode={true}
							download
							{flow}
							noSide={true}
							minHeight={graphMinHeight}
							on:select={(e) => {
								if (e.detail) {
									stepDetail = e.detail
									rightPaneSelected = 'flow_step'
								} else {
									stepDetail = undefined
									rightPaneSelected = 'saved_inputs'
								}
							}}
							on:triggerDetail={(e) => {
								detailLayout?.showTriggers()
							}}
							noBorder={true}
						/>
					</div>
				{/if}
			</div>
		{/if}
	{/snippet}
	{#snippet save_inputs()}
		<SavedInputsV2
			bind:this={savedInputsV2}
			schema={flow?.schema}
			{jsonView}
			flowPath={flow?.path}
			{isValid}
			args={args ?? {}}
			bind:inputSelected
			on:selected_args={(e) => {
				const nargs = JSON.parse(JSON.stringify(e.detail))
				setArgs(nargs)
				if (jsonView) {
					runForm?.syncJsonEditor()
				}
			}}
		/>
	{/snippet}

	{#snippet flow_step({ onBack }: { onBack?: () => void })}
		{#if flow}
			{#if stepDetail}
				<FlowGraphViewerStep schema={flow.schema} {stepDetail} {onBack} />
			{/if}
		{/if}
	{/snippet}
	{#snippet triggers()}
		{#if flow}
			<TriggersEditor
				{args}
				runnableVersion={flow.version_id?.toString()}
				initialPath={flow.path}
				currentPath={flow.path}
				noEditor={true}
				newItem={false}
				isFlow={true}
				schema={flow.schema}
				isDeployed={true}
				noCapture={true}
				isEditor={false}
			/>
		{/if}
	{/snippet}

	{#snippet flow_graph()}
		{#if flow}
			<div class="h-full overflow-auto" bind:clientHeight={flowGraphHeight}>
				<FlowGraphViewer
					triggerNode={true}
					download
					{flow}
					noSide={true}
					noBorder
					minHeight={flowGraphHeight}
					on:select={(e) => {
						if (e.detail) {
							stepDetail = e.detail
							rightPaneSelected = 'flow_step'
						} else {
							stepDetail = undefined
							rightPaneSelected = 'saved_inputs'
						}
					}}
					on:triggerDetail={(e) => {
						detailLayout?.showTriggers()
					}}
				/>
			</div>
		{/if}
	{/snippet}
</DetailPageLayout>

<FlowAssetsHandler
	modules={flow?.value.modules ?? []}
	enableDbExplore
	enablePathScriptAndFlowAssets
/>
