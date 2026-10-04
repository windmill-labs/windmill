<script lang="ts">
	import { base } from '$lib/base'
	import {
		JobService,
		ScriptService,
		WorkspaceService,
		type Script,
		type TriggersCount,
		type WorkspaceDeployUISettings
	} from '$lib/gen'
	import {
		defaultIfEmptyString,
		emptyString,
		canWrite,
		truncateHash,
		copyToClipboard,
		urlParamsToObject,
		extractTagFromSharableHash,
		interpolateTag,
		isDynamicTag,
		isTagTemplate
	} from '$lib/utils'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import {
		disableHubStore,
		enterpriseLicense,
		hubBaseUrlStore,
		userWorkspaces,
		workspaceStore
	} from '$lib/stores'
	import { isDeployable, ALL_DEPLOYABLE } from '$lib/utils_deployable'
	import AIFormAssistant from '$lib/components/copilot/AIFormAssistant.svelte'

	import { onDestroy, setContext, tick, untrack } from 'svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import {
		Tabs,
		Tab,
		TabContent,
		Badge,
		Alert,
		DrawerContent,
		Drawer,
		Button
	} from '$lib/components/common'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import RunForm from '$lib/components/RunForm.svelte'
	import DbtRunGraph from '$lib/components/dbt/DbtRunGraph.svelte'
	import { goto } from '$lib/navigation'
	import MoveDrawer from '$lib/components/MoveDrawer.svelte'

	import { sendUserToast } from '$lib/toast'
	import NoDirectDeployAlert from '$lib/components/NoDirectDeployAlert.svelte'
	import DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'

	import SavedInputsV2 from '$lib/components/SavedInputsV2.svelte'
	import DetailPageLayout from '$lib/components/details/DetailPageLayout.svelte'
	import OnBehalfOfBadge from '$lib/components/details/OnBehalfOfBadge.svelte'
	import DetailPageHeader from '$lib/components/details/DetailPageHeader.svelte'
	import {
		Activity,
		Archive,
		ArchiveRestore,
		Eye,
		FolderOpen,
		GitFork,
		Globe2,
		History,
		Loader2,
		Pen,
		ChevronUpSquare,
		Shield,
		Trash,
		Play,
		ClipboardCopy,
		LayoutDashboard,
		ChevronDown,
		ChevronRight
	} from 'lucide-svelte'
	import { scriptToHubUrl } from '$lib/hub'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import ScriptVersionHistory from '$lib/components/ScriptVersionHistory.svelte'
	import { createRawAppFromScript } from '$lib/components/details/createRawAppFromScript'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import PersistentScriptDrawer from '$lib/components/PersistentScriptDrawer.svelte'
	import GfmMarkdown from '$lib/components/GfmMarkdown.svelte'
	import Star from '$lib/components/Star.svelte'
	import LogViewer from '$lib/components/LogViewer.svelte'
	import { Highlight } from 'svelte-highlight'
	import json from 'svelte-highlight/languages/json'
	import { writable } from 'svelte/store'
	import Toggle from '$lib/components/Toggle.svelte'
	import InputSelectedBadge from '$lib/components/schema/InputSelectedBadge.svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import { slide } from 'svelte/transition'
	import TriggersBadge from '$lib/components/graph/renderers/triggers/TriggersBadge.svelte'
	import TriggersEditor from '$lib/components/triggers/TriggersEditor.svelte'
	import { Triggers } from '$lib/components/triggers/triggers.svelte'
	import { interceptNav } from '$lib/components/details/interceptNav'
	import {
		setOperatingWorkspace,
		useOperatingUser,
		useOperatingWorkspaceHref
	} from '$lib/components/operatingWorkspace.svelte'
	import {
		buildForkEditUrl,
		editInForkAllowed,
		editInForkDescription,
		editInForkLabel,
		onEditInForkClick,
		openEditInFork
	} from '$lib/utils/editInFork'
	import { isCloudHosted } from '$lib/cloud'
	import { isWorkflowAsCode } from '$lib/components/graph/wacToFlow'
	import WacDiagram from '$lib/components/graph/WacDiagram.svelte'
	import { twMerge } from 'tailwind-merge'
	import CiTestResults from '$lib/components/CiTestResults.svelte'

	let {
		hash: routeHash,
		workspace,
		searchParams,
		locationHash = '',
		onNavigate = goto,
		active = true,
		embedded = false,
		seededRun,
		seededByAgent = false,
		onSeedApplied,
		onClearSeededRun,
		onLoadState
	}: {
		/** The `[...hash]` route segment: either a script hash or a script path. */
		hash: string
		workspace: string | undefined
		/** Query of the URL this detail was opened at (`dbt_retry_from`). */
		searchParams?: URLSearchParams
		/** `#`-fragment of that URL, carrying prefilled run args from `Run again`. */
		locationHash?: string
		onNavigate?: (url: string) => void | Promise<void>
		/** Gates the window-level Ctrl+Enter handler, so a hidden instance cannot steal it. */
		active?: boolean
		/** Rendered inside a page that is not the script's own (an AI session preview tab), whose
		 * URL this must leave alone and which a cross-workspace link must not navigate away. */
		embedded?: boolean
		/** Arguments a chat tool proposed when it opened the page, for as long as they have
		 * still to be applied. Nothing waits on them: the page runs as it always does, and
		 * these only fill the fields. `seq` identifies the request, so a second one re-seeds a
		 * page already open. Gone once applied — this page remounts under its host on a
		 * deploy, a refresh or a version pin, and applying again would overwrite what the
		 * reader has typed since. */
		seededRun?: { args: Record<string, any>; seq: number }
		/** Whether to say the agent filled them. Outlives `seededRun`, since the note stands
		 * until the reader dismisses it. */
		seededByAgent?: boolean
		/** Applied, so the host stops offering them. */
		onSeedApplied?: () => void
		/** The reader dismissed the note. The values stay in the fields. */
		onClearSeededRun?: () => void
		/** How the load ended, for a host that renders its own state around this page.
		 * Providing it also suppresses the "could not load" toast: a 404 here is the normal
		 * state of a not-yet-deployed path, which the host explains in place instead. */
		onLoadState?: (state: 'loaded' | 'not_found') => void
	} = $props()

	// Every workspace-scoped call below and in the drawers this page opens targets
	// `workspace`, not the nav store: a session views an item in its own workspace.
	setOperatingWorkspace(() => workspace)

	// Roles are per-workspace, so every gate below asks about `workspace` rather than about
	// the one the browser is navigated to. Unresolved reads as unknown, which `canWrite`
	// refuses — the safe answer while a fork's `whoami` is still in flight.
	const operatingUser = useOperatingUser()
	// A ⌘-click opens these in a new tab, which only the query names the workspace for.
	const operatingHref = useOperatingWorkspaceHref()
	const actingUser = $derived(operatingUser.current)

	let script: Script | undefined = $state()
	let topHash: string | undefined = $state()
	let isHubScript = $state(false)
	// Derived, not assigned during the load: in a fork the acting user is looked up
	// asynchronously, and a value read after `getScriptByHash` resolves can land before that
	// lookup does. Assigned once, that leaves the page permanently read-only — `canWrite`
	// refuses an unknown user — with nothing to recompute it. A hub script is never writable.
	const can_write = $derived(
		!isHubScript &&
			!!script &&
			script.workspace_id == workspace &&
			canWrite(script.path, script.extra_perms!, actingUser)
	)
	let deploymentInProgress = $state(false)
	let expandedModuleLocks: Record<string, boolean> = $state({})
	let expandedModuleCode: Record<string, boolean> = $state({})
	let deploymentJobId: string | undefined = $state(undefined)
	let intervalId: number
	let shareModal: ShareModal | undefined = $state()
	let runForm: RunForm | undefined = $state()

	let scheduledForStr: string | undefined = $state(undefined)
	let invisible_to_owner: boolean | undefined = $state(undefined)
	let overrideTag: string | undefined = $state(undefined)
	let overrideTagNote: string | undefined = $state(undefined)
	// Tag carried over from 'Run again', pending the dynamic-tag check in loadScript
	let carriedTag: string | undefined = undefined
	let inputSelected: 'saved' | 'history' | undefined = $state(undefined)
	let jsonView = $state(false)

	let previousHash: string | undefined = $state(undefined)

	let isWac = $derived(
		script?.content && script?.language ? isWorkflowAsCode(script.content, script.language) : false
	)

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

	async function deleteScript(hash: string): Promise<void> {
		try {
			await ScriptService.deleteScriptByHash({ workspace: workspace!, hash })
			loadScript(hash)
		} catch (err) {
			console.error(err)
			sendUserToast(`Could not delete this script ${err.body}`, true)
		}
	}

	async function archiveScript(hash: string): Promise<void> {
		await ScriptService.archiveScriptByHash({ workspace: workspace!, hash })
		loadScript(hash)
	}

	async function unarchiveScript(hash: string): Promise<void> {
		const r = await ScriptService.getScriptByHash({ workspace: workspace!, hash })
		const ns = await ScriptService.createScript({
			workspace: workspace!,
			requestBody: {
				...r,
				parent_hash: hash,
				lock: r.lock
			}
		})
		sendUserToast(`Unarchived script`)
		loadScript(ns)
		onNavigate(`/scripts/get/${ns}`)
	}

	async function syncer(): Promise<void> {
		if (script?.hash) {
			const status = await ScriptService.getScriptDeploymentStatus({
				workspace: workspace!,
				hash: script?.hash!
			})
			if (status.lock != undefined || status.lock_error_logs != undefined) {
				deploymentInProgress = false
				deploymentJobId = undefined
				script.lock = status.lock
				script.lock_error_logs = status.lock_error_logs
				clearInterval(intervalId)
			} else if (status.job_id) {
				deploymentJobId = status.job_id
			}
		}
	}

	async function loadTriggers(path: string): Promise<void> {
		await triggersState.fetchTriggers(triggersCount, workspace, path, false, undefined, actingUser)
	}

	async function loadScript(hash: string): Promise<void> {
		// Check if this is a hub script path
		if (hash.startsWith('hub/')) {
			isHubScript = true
			try {
				const hubScript = await ScriptService.getHubScriptByPath({ path: hash })
				// Create a partial Script object from hub script data
				script = {
					hash: '',
					path: hash,
					summary: hubScript.summary ?? '',
					description: '',
					content: hubScript.content,
					created_by: '',
					created_at: '',
					archived: false,
					deleted: false,
					is_template: false,
					extra_perms: {},
					lock: hubScript.lockfile,
					language: hubScript.language as Script['language'],
					kind: 'script',
					starred: false,
					schema: hubScript.schema as Script['schema'],
					auto_kind: undefined,
					has_preprocessor: false
				}
				return
			} catch (e) {
				sendUserToast('Could not load hub script: ' + e.body, true)
				return
			}
		}

		isHubScript = false
		try {
			script = await ScriptService.getScriptByHash({
				workspace: workspace!,
				hash,
				withStarredInfo: true,
				authed: true
			})
		} catch {
			try {
				script = await ScriptService.getScriptByPath({
					workspace: workspace!,
					path: hash,
					withStarredInfo: true
				})
				hash = script.hash
			} catch (e) {
				// A path with no deployed script 404s here, which is the normal state of a
				// draft the user has not deployed — the host says so rather than the page
				// staying blank, so no toast for it. Anything else is a real failure.
				if (onLoadState && e?.status === 404) onLoadState('not_found')
				else sendUserToast('Could not load script: ' + (e?.body ?? e?.message ?? e), true)
				return
			}
		}
		// A carried non-template value equal to the resolution of the script's own dynamic
		// tag for these args is not an override but the previous run's pinned resolution:
		// drop it so the backend re-resolves from the (possibly edited) args. A differing
		// value is a genuine user override and a template re-resolves at push, so both stay.
		if (
			carriedTag &&
			isDynamicTag(script.tag) &&
			!isTagTemplate(carriedTag) &&
			interpolateTag(script.tag ?? '', workspace!, args) === carriedTag
		) {
			if (overrideTag === carriedTag) {
				overrideTag = undefined
				overrideTagNote = `tag ${script.tag} is resolved at run time, so the previous run's tag ${carriedTag} was not applied`
			}
			carriedTag = undefined
		}
		loadTriggers(script.path)

		if (script.path && script.archived) {
			const script_by_path = await ScriptService.getScriptByPath({
				workspace: workspace!,
				path: script.path
			}).catch((_) => console.error('this script has no non-archived version'))
			if (script_by_path?.hash != script.hash) {
				topHash = script_by_path?.hash
			}
		} else {
			topHash = undefined
		}
		intervalId && clearInterval(intervalId)
		deploymentInProgress = script.lock == undefined && script.lock_error_logs == undefined
		if (deploymentInProgress) {
			intervalId = setInterval(syncer, 500)
		}
		if (!script.path.startsWith(`u/${actingUser?.username}`) && script.path.split('/').length > 2) {
			invisible_to_owner = script.visible_to_runner_only
		}
		onLoadState?.('loaded')
	}

	onDestroy(() => {
		intervalId && clearInterval(intervalId)
	})

	let isValid = $state(true)

	let runLoading = $state(false)
	async function runScript(
		scheduledForStr: string | undefined,
		args: Record<string, any>,
		invisibleToOwner: boolean | undefined,
		overrideTag: string | undefined
	) {
		try {
			runLoading = true
			const scheduledFor = scheduledForStr ? new Date(scheduledForStr).toISOString() : undefined
			let run: string
			if (isHubScript) {
				run = await JobService.runScriptByPath({
					workspace: workspace!,
					path: script?.path ?? '',
					requestBody: args,
					scheduledFor,
					invisibleToOwner,
					tag: overrideTag,
					skipPreprocessor: true
				})
			} else {
				run = await JobService.runScriptByHash({
					workspace: workspace!,
					hash: script?.hash ?? '',
					requestBody: args,
					scheduledFor,
					invisibleToOwner,
					tag: overrideTag,
					skipPreprocessor: true
				})
			}
			await onNavigate('/run/' + run + '?workspace=' + workspace)
		} catch (err) {
			sendUserToast(`Could not create job: ${err.body}`, true)
		} finally {
			// Not only on failure: a navigation that keeps this component mounted — the run
			// opens as another preview tab in a session — would leave Run spinning forever.
			runLoading = false
		}
	}

	let args: Record<string, any> | undefined = $state(undefined)

	// Seeded once the form exists, and again whenever another request arrives for a page
	// already open — latched on the request rather than on "seeded once", which would leave
	// the previous turn's values on screen. The arguments were narrowed against this script's
	// deployed schema before they got here, by the tool that opened the page.
	let seededSeq: number | undefined = undefined
	$effect(() => {
		if (!seededRun || !runForm) return
		if (seededSeq === seededRun.seq) return
		seededSeq = seededRun.seq
		runForm.setArgs(seededRun.args)
		onSeedApplied?.()
	})

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
			args = Object.fromEntries(params)
		} catch (e) {
			console.error('Was not able to transform hash as args', e)
		}
	}

	// A dbt retry must name the run it resumes, and a job id is not something to
	// type from memory: picking `retry` fills the field with the run this caller's
	// retry would land on. Filled once per script, so clearing it stays cleared.
	let dbtResumableAsked: string | undefined = $state(undefined)
	$effect(() => {
		const ws = workspace
		const path = script?.path
		const command = args?.['command'] as Record<string, any> | undefined
		if (!ws || !path || script?.language !== 'dbt' || command?.label !== 'retry') return
		if (command['dbt_retry_job'] || dbtResumableAsked === path) return
		dbtResumableAsked = path
		JobService.getDbtResumableForScript({ workspace: ws, path })
			.then((held) => {
				// The page may show another script, or another workspace, by now:
				// filling THIS answer into that form aims its retry at a run of a
				// script it is not, which only a reload could undo.
				if (workspace !== ws || script?.path !== path) return
				const current = untrack(() => args)
				const block = current?.['command'] as Record<string, any> | undefined
				if (!held || !current || block?.label !== 'retry' || block['dbt_retry_job']) return
				args = { ...current, command: { ...block, dbt_retry_job: held } }
				if (jsonView) {
					runForm?.syncJsonEditor()
				}
			})
			.catch(() => {})
	})

	// Arrived from a failed dbt run's `Run again`, which prefills the arguments it
	// ran with — a whole-project rebuild. Resuming that run instead is usually
	// what the reader wants and there is nothing on this form that says so, hence
	// the offer. Confirmed against the run rather than trusted from the URL: only
	// the latest failure of a script is resumable, and by the time the form opens
	// another run may hold it.
	let dbtRetryFrom: string | undefined = $state(undefined)
	$effect(() => {
		const ws = workspace
		const from = searchParams?.get('dbt_retry_from') ?? undefined
		dbtRetryFrom = undefined
		if (!ws || !from || script?.language !== 'dbt') return
		JobService.getDbtResumable({ workspace: ws, id: from })
			.then((held) => {
				if (workspace === ws && held === from) dbtRetryFrom = from
			})
			.catch(() => {})
	})

	function useDbtRetry() {
		const from = dbtRetryFrom
		if (!from) return
		// Merged, never replaced: a tag, concurrency key or debounce key may
		// interpolate `command.vars`, and the QUEUE reads those at enqueue — long
		// before the worker restores the failed run's own arguments. Dropping them
		// here routes the retry by a different key than the run it resumes. Same
		// as the run page's retry.
		args = {
			...(args ?? {}),
			command: {
				...((args?.['command'] as Record<string, any> | undefined) ?? {}),
				label: 'retry',
				dbt_retry_job: from
			}
		}
		if (jsonView) {
			runForm?.syncJsonEditor()
		}
	}

	let moveDrawer: MoveDrawer | undefined = $state()
	let deploymentDrawer: DeployWorkspaceDrawer | undefined = $state()
	let persistentScriptDrawer: PersistentScriptDrawer | undefined = $state()
	let showEditButtons = $state(false)

	// The dev workspace's editor is not one the session panel can host, so from a preview tab
	// it opens in a new browser tab, as the session editors' own entry does.
	function editInFork(e: Event | undefined, itemPath: string) {
		if (!embedded) {
			return onEditInForkClick(e, 'script', itemPath, { hasHref: true, prodWorkspace: workspace })
		}
		const m = e as MouseEvent | undefined
		if (m && (m.metaKey || m.ctrlKey || m.shiftKey || m.altKey || (m.button ?? 0) !== 0)) return
		e?.preventDefault()
		return openEditInFork('script', itemPath, workspace)
	}

	function getMainButtons(
		script: Script | undefined,
		args: object | undefined,
		topHash?: string,
		can_write?: boolean
	) {
		const buttons: any = []

		if (!topHash && script && !actingUser?.operator && !script.codebase) {
			buttons.push({
				label: 'Fork',
				description: `Start a new script from a copy of this one`,
				narrow: { dropdownOf: 'Edit' },
				buttonProps: {
					href: operatingHref(`${base}/scripts/add?template=${script.path}`),
					onClick: interceptNav(onNavigate, `/scripts/add?template=${script.path}`),
					unifiedSize: 'md',
					variant: 'subtle',
					disabled: !showEditButtons,
					startIcon: GitFork
				}
			})
		}

		if (
			script &&
			!actingUser?.operator &&
			!isCloudHosted() &&
			editInForkAllowed(workspace, $userWorkspaces)
		) {
			buttons.push({
				label: editInForkLabel(workspace, $userWorkspaces),
				description: editInForkDescription('script', workspace, $userWorkspaces),
				narrow: { dropdownOf: 'Edit' },
				buttonProps: {
					href: buildForkEditUrl('script', script.path, workspace),
					onClick: (e: Event | undefined) => editInFork(e, script.path),
					unifiedSize: 'md',
					variant: !showEditButtons ? 'default' : 'subtle',
					startIcon: Pen
				}
			})
		}

		if (!script) {
			return buttons
		}

		buttons.push({
			label: `Runs`,
			buttonProps: {
				href: operatingHref(`${base}/runs/${script.path}`),
				onClick: interceptNav(onNavigate, `/runs/${script.path}`),
				unifiedSize: 'md',
				variant: 'subtle',
				startIcon: Play
			}
		})

		if (!script || actingUser?.operator || !can_write) {
			return buttons
		}

		if (Array.isArray(script.parent_hashes) && script.parent_hashes.length > 0) {
			buttons.push({
				label: `History`,
				narrow: 'menu',
				buttonProps: {
					onClick: () => {
						versionsDrawerOpen = !versionsDrawerOpen
					},

					unifiedSize: 'md',
					variant: 'subtle',
					startIcon: History
				}
			})
		}

		if (!actingUser?.operator) {
			buttons.push({
				label: 'Build app',
				narrow: 'menu',
				buttonProps: {
					onClick: async () => {
						const app = createRawAppFromScript(script.path, script.summary, script.schema)
						// The raw app editor reads the payload from sessionStorage on mount.
						sessionStorage.setItem('rawAppImport', JSON.stringify(app))
						await onNavigate('/apps_raw/add')
					},
					disabled: !showEditButtons,
					unifiedSize: 'md',
					variant: 'subtle',
					startIcon: LayoutDashboard
				}
			})

			if (script?.restart_unless_cancelled ?? false) {
				buttons.push({
					label: 'Current runs',
					buttonProps: {
						onClick: () => {
							persistentScriptDrawer?.open?.(script)
						},
						unifiedSize: 'md',
						startIcon: Activity,
						variant: 'accent'
					}
				})
			}

			if (!script.codebase) {
				// One destination for both paths: the intercepted click has to carry the version
				// the page is showing, or Edit on a historical version opens the current script.
				const editUrl = `/scripts/edit/${script.path}${
					topHash ? `?hash=${script.hash}&topHash=${topHash}` : ''
				}`
				buttons.push({
					label: 'Edit',
					buttonProps: {
						href: operatingHref(`${base}${editUrl}`),
						onClick: interceptNav(onNavigate, editUrl),
						unifiedSize: 'md',
						startIcon: Pen,
						variant: 'accent',
						disabled: !can_write || !showEditButtons
					}
				})
			}
		}

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
		script: Script | undefined,
		deployUiSettings: WorkspaceDeployUISettings | undefined
	) {
		if (!script || actingUser?.operator) return []

		const menuItems: any = []

		if (showEditButtons) {
			menuItems.push({
				label: 'Move/Rename',
				Icon: FolderOpen,
				onclick: () => {
					moveDrawer?.openDrawer(script?.path ?? '', script?.summary, 'script')
				}
			})
		}

		menuItems.push({
			label: 'Audit logs',
			Icon: Eye,
			onclick: () => {
				onNavigate(`/audit_logs?resource=${script?.path}`)
			}
		})

		menuItems.push({
			label: 'Permissions',
			Icon: Shield,
			onclick: () => {
				shareModal?.openDrawer(script?.path ?? '', 'script')
			}
		})

		if (!$disableHubStore) {
			menuItems.push({
				label: 'Publish to Hub',
				Icon: Globe2,
				onclick: () => {
					if (!script) return
					window.open(scriptToHubUrl(script, $hubBaseUrlStore).toString(), '_blank', 'noopener')
				}
			})
		}

		if (isDeployable('script', script?.path ?? '', deployUiSettings)) {
			menuItems.push({
				label: 'Deploy to staging/prod',
				Icon: ChevronUpSquare,
				onclick: () => {
					deploymentDrawer?.openDrawer(script?.path ?? '', 'script')
				}
			})
		}

		if (showEditButtons) {
			if (script.archived) {
				menuItems.push({
					label: 'Unarchive',
					Icon: ArchiveRestore,
					onclick: async () => {
						unarchiveScript(script.hash)
					},
					color: 'red'
				})
			} else {
				menuItems.push({
					label: 'Archive',
					Icon: Archive,
					onclick: async () => {
						archiveScript(script.hash)
					},
					color: 'red'
				})
			}

			if (actingUser?.is_admin) {
				menuItems.push({
					label: 'Delete',
					Icon: Trash,
					onclick: async () => {
						deleteScript(script.hash)
					},
					color: 'red'
				})
			}
		}

		return menuItems
	}

	let versionsDrawerOpen = $state(false)

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

	let rightPaneSelected = $state('saved_inputs')

	let savedInputsV2: SavedInputsV2 | undefined = $state(undefined)
	$effect(() => {
		const cliTrigger = triggersState.triggers.find((t) => t.type === 'cli')
		if (cliTrigger) {
			cliTrigger.extra = {
				cliCommand: `wmill script run ${script?.path} -d '${JSON.stringify(args)}'`
			}
		}
	})
	let loading = $derived(!script)
	// A pinned version rides in the query rather than replacing the route segment, so a
	// host keying on the path — a session preview tab — keeps pointing at the script
	// while showing one of its old versions.
	let requestedHash = $derived(searchParams?.get('version') || routeHash)
	$effect(() => {
		if (workspace) {
			if (previousHash != requestedHash) {
				previousHash = requestedHash
				untrack(() => loadScript(requestedHash ?? ''))
			}
		}
	})
	let mainButtons = $derived(getMainButtons(script, args, topHash, can_write))
</script>

<MoveDrawer
	bind:this={moveDrawer}
	on:update={async (e) => {
		await onNavigate('/scripts/get/' + e.detail + `?workspace=${workspace}`)
		loadScript(routeHash ?? '')
	}}
/>

<svelte:window onkeydown={onKeyDown} />

<DeployWorkspaceDrawer bind:this={deploymentDrawer} />
<PersistentScriptDrawer bind:this={persistentScriptDrawer} />
<ShareModal bind:this={shareModal} />

<Drawer bind:open={versionsDrawerOpen} size="1200px">
	<DrawerContent title="Versions History" on:close={() => (versionsDrawerOpen = false)} noPadding>
		{#if script}
			<ScriptVersionHistory
				scriptPath={script.path}
				openDetails
				on:openDetails={(e) => {
					if (script) {
						onNavigate(`/scripts/get/${e.detail.version}?workspace=${workspace}`)
					}
					versionsDrawerOpen = false
				}}
			/>
		{/if}
	</DrawerContent>
</Drawer>
{#key script?.hash}
	<DetailPageLayout bind:selected={rightPaneSelected} isOperator={actingUser?.operator}>
		{#snippet header({ wide }: { wide: boolean })}
			<DetailPageHeader
				{wide}
				{mainButtons}
				menuItems={getMenuItems(script, deployUiSettings)}
				bind:errorHandlerMuted={
					() => script?.ws_error_handler_muted ?? false,
					(v) => {
						if (script !== undefined) script.ws_error_handler_muted = v
					}
				}
				errorHandlerKind="script"
				scriptOrFlowPath={script?.path ?? ''}
				tag={script?.tag ?? ''}
				labels={script?.labels}
				inheritedLabels={script?.inherited_labels}
				on:seeTriggers={() => {
					rightPaneSelected = 'triggers'
				}}
				summary={script?.summary}
				path={script?.path}
				onSaved={can_write
					? async (newPath) => {
							if (newPath !== script?.path) {
								await onNavigate(`/scripts/get/${newPath}?workspace=${workspace}`)
							} else {
								loadScript(newPath)
							}
						}
					: undefined}
			>
				{#snippet trigger_badges()}
					<TriggersBadge
						showOnlyWithCount={true}
						showDraft={false}
						path={script?.path ?? ''}
						newItem={false}
						isFlow={false}
						selected={rightPaneSelected === 'triggers'}
						onSelect={async (triggerIndex: number) => {
							if (rightPaneSelected !== 'triggers') {
								rightPaneSelected = 'triggers'
							}
							await tick()
							triggersState.selectedTriggerIndex = triggerIndex
						}}
					/>
				{/snippet}
				{#if workspace && script}
					<!-- Favorites are the sidebar's, which lists the navigation workspace only. -->
					{#if workspace === $workspaceStore}
						<Star kind="script" path={script.path} summary={script.summary} />
					{/if}
				{/if}
				{#if script?.auto_kind === 'wac'}
					<Popover notClickable>
						{#snippet text()}
							Workflow-as-Code
						{/snippet}
						<Badge small color="indigo" baseClass="border border-indigo-200">wac</Badge>
					</Popover>
				{/if}
				{#if script?.auto_kind === 'test'}
					<Popover notClickable>
						{#snippet text()}
							CI test script
						{/snippet}
						<Badge small color="yellow" baseClass="border">CI test</Badge>
					</Popover>
				{/if}
				{#if script?.codebase}
					<Badge
						>bundle<Tooltip
							>This script is deployed as a bundle and can only be deployed from the CLI for now</Tooltip
						></Badge
					>
				{/if}
				<OnBehalfOfBadge
					onBehalfOf={script?.on_behalf_of}
					onBehalfOfEmail={script?.on_behalf_of_email}
					kind="script"
				/>
				{#if script?.priority != undefined}
					<div class="hidden md:block">
						<Badge color="blue" variant="outlined" size="xs">
							{`Priority: ${script.priority}`}
						</Badge>
					</div>
				{/if}
				{#if script?.restart_unless_cancelled ?? false}
					<button onclick={() => persistentScriptDrawer?.open?.(script)}>
						<div class="hidden md:block">
							<Badge color="red" variant="outlined" size="xs">Persistent</Badge>
						</div>
					</button>
				{/if}
				{#if script?.concurrent_limit != undefined && script.concurrency_time_window_s != undefined}
					<div class="hidden md:block">
						<Badge color="gray" variant="outlined" size="xs">
							{`Concurrency limit: ${script.concurrent_limit} runs every ${script.concurrency_time_window_s}s`}
						</Badge>
					</div>
				{/if}
			</DetailPageHeader>
		{/snippet}
		{#snippet form()}
			<div class="px-3">
				<NoDirectDeployAlert onUpdateCanEditStatus={(v) => (showEditButtons = v)} />
			</div>
			{#if script}
				{@const showsDbtGraph = script.language === 'dbt' && !!script.path}
				<div class={twMerge('flex flex-col', isWac || showsDbtGraph ? 'h-full divide-y' : '')}>
					<div
						class={twMerge(
							'p-8 w-full max-w-3xl overflow-y-auto mx-auto flex flex-col relative',
							isWac ? 'max-h-1/2' : '',
							// The graph takes the rest of the pane, as a flow's does, so the form
							// scrolls within its half instead of pushing the models off-screen.
							showsDbtGraph ? 'shrink-0 max-h-1/2' : ''
						)}
					>
						{#if script?.path}
							<CiTestResults path={script.path} kind="script" />
						{/if}

						<div class="flex flex-col gap-0.5 mb-1">
							{#if script.lock_error_logs || topHash || script.archived || script.deleted}
								<div class="flex flex-col gap-2 my-2">
									{#if script.lock_error_logs}
										<Alert type="error" title="Deployment failed">
											<p>
												This script has not been deployed successfully because of the following
												errors:
											</p>
											<LogViewer
												content={script.lock_error_logs}
												isLoading={false}
												tag={undefined}
											/>
										</Alert>
									{/if}
									{#if topHash}
										<div class="mt-2"></div>
										<Alert type="warning" title="Not HEAD">
											This hash is not HEAD (latest non-archived version at this path) :
											<a
												href="{base}/scripts/get/{topHash}?workspace={workspace}"
												onclick={interceptNav(onNavigate, `/scripts/get/${topHash}`)}
												>Go to the HEAD of this path</a
											>
										</Alert>
									{/if}
									{#if script.archived && !topHash}
										<Alert type="error" title="Archived">This path was archived</Alert>
									{/if}
									{#if script.deleted}
										<Alert type="error" title="Deleted">
											<p>The content of this script was deleted (by an admin, no less)</p>
										</Alert>
									{/if}
								</div>
							{/if}

							{#if !emptyString(script.description)}
								<div class="p-4 rounded-md bg-surface-secondary">
									<GfmMarkdown
										md={defaultIfEmptyString(script?.description, 'No description')}
										noPadding
									/>
								</div>
								<div class="h-4"></div>
							{/if}
						</div>

						{#if deploymentInProgress}
							<div class="pb-4" transition:slide={{ duration: 150 }}>
								<Badge color="yellow">
									<Loader2 size={12} class="inline animate-spin mr-1" />
									Deployment in progress
									{#if deploymentJobId}
										<a
											href="/run/{deploymentJobId}?workspace={workspace}"
											class="underline"
											target="_blank">view job</a
										>
									{/if}
								</Badge>
							</div>
						{/if}

						<div class="flex flex-col align-left">
							{#if (script.schema && Object.keys(script.schema.properties ?? {}).length > 0) || inputSelected}
								{@const hasSchema =
									script.schema && Object.keys(script.schema.properties ?? {}).length > 0}
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
									{#if hasSchema}
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

							<!-- Landed here from a failed dbt run's `Run again`, which prefills a
							     whole-project rebuild. Resuming that run is one click, and the
							     form alone never says it is possible. -->
							{#if dbtRetryFrom && (args?.['command'] as any)?.label !== 'retry'}
								<div class="mb-2">
									<Alert type="info" size="xs" title="This rebuilds the whole project">
										<div class="flex flex-row gap-2 items-center flex-wrap">
											<span>
												The run you came from failed part-way. <span class="font-mono"
													>dbt retry</span
												>
												rebuilds only its failed and skipped nodes, with the arguments it ran with.
											</span>
											<Button size="xs" variant="border" color="light" on:click={useDbtRetry}>
												Retry that run instead
											</Button>
										</div>
									</Alert>
								</div>
							{/if}

							{#if Object.keys(script?.schema?.properties ?? {}).length > 0}
								<AIFormAssistant
									instructions={(script?.schema?.prompt_for_ai as string | undefined) ?? ''}
									onEditInstructions={can_write && !actingUser?.operator
										? () => onNavigate(`/scripts/edit/${script?.path}?metadata_open=true`)
										: undefined}
									runnableType="script"
									path={script?.path}
								/>
							{/if}

							{#if seededByAgent}
								<InputSelectedBadge inputSelected="agent" onReject={() => onClearSeededRun?.()} />
							{/if}
							<RunForm
								bind:scheduledForStr
								bind:invisible_to_owner
								bind:overrideTag
								{overrideTagNote}
								syncArgsToUrl={!embedded}
								viewKeybinding
								loading={runLoading}
								autofocus
								detailed={false}
								bind:isValid
								runnable={script}
								runAction={runScript}
								bind:args
								schedulable={true}
								bind:this={runForm}
								{jsonView}
							/>
						</div>

						<div class="pt-4 flex flex-row gap-1 w-full justify-end items-center">
							{#if !isHubScript}
								<span class="text-2xs text-secondary">
									Edited <TimeAgo date={script.created_at || ''} /> by {script.created_by ||
										'unknown'}
								</span>
							{/if}
							<div class="flex flex-row gap-x-2 flex-wrap items-center">
								{#if !isHubScript}
									<Badge small color="gray">
										{truncateHash(script?.hash ?? '')}
									</Badge>
								{/if}
								{#if script?.is_template}
									<Badge color="blue">Template</Badge>
								{/if}
								{#if script && script.kind !== 'script'}
									<Badge color="blue">
										{script?.kind}
									</Badge>
								{/if}

								<SharedBadge canWrite={can_write} extraPerms={script?.extra_perms ?? {}} />
							</div>
						</div>
					</div>
					{#if showsDbtGraph}
						<!-- The bottom of the pane, the way a flow's graph takes it: a dbt
						     script's `content` is a descriptor, so what it BUILDS is the thing
						     to look at. Pinned to the version on screen and given no job, so
						     this is the project as THIS deploy declared it; a node opens its
						     SQL and previews its rows with whatever the form above holds. -->
						<div class="grow min-h-0">
							<DbtRunGraph scriptPath={script.path} scriptHash={script.hash} runArgs={args} fill />
						</div>
					{:else if isWac && script.content}
						<div class="grow min-h-0" style="min-height: 400px;">
							<WacDiagram code={script.content} language={script.language ?? ''} />
						</div>
					{/if}
				</div>
			{/if}
		{/snippet}
		{#snippet save_inputs()}
			{#if args}
				<SavedInputsV2
					schema={script?.schema}
					bind:this={savedInputsV2}
					scriptPath={script?.path}
					scriptHash={topHash}
					{isValid}
					{jsonView}
					{args}
					bind:inputSelected
					on:selected_args={(e) => {
						const nargs = JSON.parse(JSON.stringify(e.detail))
						args = nargs
						if (jsonView) {
							runForm?.syncJsonEditor()
						}
					}}
				/>
			{/if}
		{/snippet}
		{#snippet triggers()}
			{#if script}
				<TriggersEditor
					{args}
					runnableVersion={script.hash}
					initialPath={script.path}
					currentPath={script.path}
					noEditor={true}
					newItem={false}
					isFlow={false}
					schema={script.schema}
					isDeployed={true}
					noCapture={true}
					isEditor={false}
				/>
			{/if}
		{/snippet}
		{#snippet scriptRender()}
			<div class="h-full">
				<Skeleton {loading} layout={[[20]]} />

				<Tabs selected="code">
					<Tab value="code" label="Code" />
					<Tab value="dependencies" label="Lockfile" />
					<Tab value="schema" label="Schema" />
					{#snippet content()}
						{#if script}
							<TabContent value="code">
								<div class="p-2 w-full">
									<HighlightCode
										language={script.language}
										code={script.content}
										className="whitespace-pre-wrap"
									/>
								</div>
								{#if script?.modules}
									{#each Object.entries(script.modules) as [modulePath, mod]}
										<div class="mt-2 border rounded mx-2">
											<button
												class="flex items-center gap-1 w-full px-3 py-2 text-sm font-medium text-secondary hover:bg-surface-hover"
												onclick={() =>
													(expandedModuleCode[modulePath] = !expandedModuleCode[modulePath])}
											>
												{#if expandedModuleCode[modulePath]}
													<ChevronDown size={14} />
												{:else}
													<ChevronRight size={14} />
												{/if}
												{modulePath}
											</button>
											{#if expandedModuleCode[modulePath]}
												<div class="p-2 w-full">
													<HighlightCode
														language={mod.language}
														code={mod.content}
														className="whitespace-pre-wrap"
													/>
												</div>
											{/if}
										</div>
									{/each}
								{/if}
							</TabContent>
							<TabContent value="dependencies">
								<div>
									{#if script?.lock}
										<div class="relative overflow-x-auto w-full">
											<Button
												wrapperClasses="absolute top-2 right-2 z-20"
												on:click={() => copyToClipboard(script?.lock)}
												color="light"
												size="xs2"
												startIcon={{
													icon: ClipboardCopy
												}}
												iconOnly
											/>
											<pre class="bg-surface-secondary text-sm p-2 h-full overflow-auto w-full"
												>{script.lock}</pre
											>
										</div>
									{:else}
										<p class="bg-surface-secondary text-sm p-2">
											There is no lock file for this script
										</p>
									{/if}
									{#if script?.modules}
										{@const moduleEntries = Object.entries(script.modules).filter(
											([_, m]) => m.lock
										)}
										{#each moduleEntries as [modulePath, mod]}
											<div class="mt-2 border rounded">
												<button
													class="flex items-center gap-1 w-full px-3 py-2 text-sm font-medium text-secondary hover:bg-surface-hover"
													onclick={() =>
														(expandedModuleLocks[modulePath] = !expandedModuleLocks[modulePath])}
												>
													{#if expandedModuleLocks[modulePath]}
														<ChevronDown size={14} />
													{:else}
														<ChevronRight size={14} />
													{/if}
													{modulePath}
												</button>
												{#if expandedModuleLocks[modulePath]}
													<div class="relative overflow-x-auto w-full">
														<Button
															wrapperClasses="absolute top-2 right-2 z-20"
															on:click={() => copyToClipboard(mod.lock ?? '')}
															color="light"
															size="xs2"
															startIcon={{
																icon: ClipboardCopy
															}}
															iconOnly
														/>
														<pre
															class="bg-surface-secondary text-sm p-2 h-full overflow-auto w-full"
															>{mod.lock}</pre
														>
													</div>
												{/if}
											</div>
										{/each}
									{/if}
								</div>
							</TabContent>
							<TabContent value="schema">
								<div class="p-1 relative h-full">
									<button
										onclick={() => copyToClipboard(JSON.stringify(script?.schema, null, 4))}
										class="absolute top-2 right-2"
									>
										<ClipboardCopy size={14} />
									</button>
									<Highlight language={json} code={JSON.stringify(script?.schema, null, 4)} />
								</div>
							</TabContent>
						{/if}
					{/snippet}
				</Tabs>
			</div>

			{#if script?.envs && script.envs.length > 0}
				<h3>Static Env Variables</h3>
				<ul>
					{#each script?.envs as e}
						<li>{e}</li>
					{/each}
				</ul>
			{/if}
		{/snippet}
	</DetailPageLayout>
{/key}
