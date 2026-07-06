<script lang="ts">
	import { run } from 'svelte/legacy'

	import {
		JobService,
		ResourceService,
		SettingService,
		UserService,
		VariableService,
		WorkspaceService,
		type AIProvider,
		type CompletedJob
	} from '$lib/gen'
	import { validateUsername } from '$lib/utils'
	import { logoutWithRedirect } from '$lib/logoutKit'
	import { page } from '$app/state'
	import { usersWorkspaceStore, userWorkspaces, workspaceStore } from '$lib/stores'
	import {
		workspaceIsFork,
		findWorkspaceRoot,
		findWorkspaceDescendants
	} from '$lib/utils/workspaceHierarchy'
	import { resource } from 'runed'
	import { Button } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { onMount } from 'svelte'
	import { sendUserToast } from '$lib/toast'
	import TestAIKey from '$lib/components/copilot/TestAIKey.svelte'
	import { switchWorkspace } from '$lib/storeUtils'
	import { deleteSessionsForWorkspace } from '$lib/components/sessions/sessionState.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { AI_PROVIDERS } from '$lib/components/copilot/lib'
	import { LoaderCircle, Trash2 } from 'lucide-svelte'
	import PrefixedInput from '../PrefixedInput.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import { jobManager } from '$lib/services/JobManager'
	import Alert from '../common/alert/Alert.svelte'
	import { base } from '$lib/base'
	import Label from '../Label.svelte'
	import ForkDatatableSection from './ForkDatatableSection.svelte'
	import Select from '../select/Select.svelte'
	import WorkspaceScopeTrigger from '../WorkspaceScopeTrigger.svelte'
	import DarkModeToggle from '../sidebar/DarkModeToggle.svelte'
	import ForkDucklakeSection from './ForkDucklakeSection.svelte'

	interface Props {
		isFork?: boolean
		// Rendered inside the global fork modal rather than the standalone
		// /user/create_workspace page: content-driven height and no
		// "Back to workspaces" navigation.
		inModal?: boolean
		onFinish?: () => void
	}

	let { isFork = false, inModal = false, onFinish }: Props = $props()

	// Dev-workspace mode: create the fork as a persistent, prefix-less dev workspace and (optionally)
	// lock the parent ("prod") against direct edits.
	let createAsDevWorkspace = $state(false)
	let lockProdDeploy = $state(true)
	let lockProdForking = $state(true)
	// Bring the parent's members into the fork (a shared env). Defaults on for a
	// dev workspace, off for a throwaway fork; flipping the dev toggle resets it.
	let copyMembers = $state(false)
	$effect(() => {
		copyMembers = createAsDevWorkspace
	})
	// A dev workspace can only be created off a root base (backend rejects a dev of a fork). Clear a
	// stale toggle when the base no longer qualifies so we never submit is_dev_workspace against a fork.
	$effect(() => {
		if (!canDesignateDevWorkspace && createAsDevWorkspace) {
			createAsDevWorkspace = false
		}
	})

	// The base workspace to fork from. A fork's git branch is based on its parent's branch, so picking
	// a fork here (rather than the root) yields a fork of a fork.
	let baseWorkspaceId = $state<string | undefined>(undefined)

	// Base candidates are the current workspace's family: its root first, then every fork/dev under it.
	let familyRoot = $derived(findWorkspaceRoot($workspaceStore, $userWorkspaces))
	let baseCandidates = $derived(
		familyRoot ? [familyRoot, ...findWorkspaceDescendants(familyRoot.id, $userWorkspaces)] : []
	)
	let baseItems = $derived(
		baseCandidates.map((w) => ({
			value: w.id,
			label: w.id === familyRoot?.id ? `${w.name} (root)` : w.name,
			subtitle: w.is_dev_workspace ? 'dev workspace' : w.id === familyRoot?.id ? undefined : 'fork'
		}))
	)
	let defaultBaseWorkspaceId = $derived(familyRoot?.id)
	// Seed the base once the family is known; keep an explicit user choice as long as it stays valid.
	$effect(() => {
		if (!isFork) return
		if (baseWorkspaceId && baseCandidates.some((w) => w.id === baseWorkspaceId)) return
		baseWorkspaceId = defaultBaseWorkspaceId
	})

	// The dev-workspace option is only offered when forking a root workspace that doesn't already
	// have one: a workspace gets at most one dev, and dev workspaces don't nest (a dev of a dev).
	let baseWorkspaceEntry = $derived($userWorkspaces.find((w) => w.id === baseWorkspaceId))
	// Require the base workspace to be loaded before treating it as a root: a missing entry must
	// not read as root (it would offer invalid dev creation while the workspace list is still loading).
	// `workspaceIsFork` (prefix OR parent) also excludes an orphaned `wm-fork-` workspace, whose parent
	// FK was set null — it has no parent but is still a fork, so it can't host a dev workspace.
	let currentIsRoot = $derived(
		!!baseWorkspaceEntry && !workspaceIsFork(baseWorkspaceId, $userWorkspaces)
	)
	// Ask the server whether a dev already exists: the caller may not be a member of this prod's dev,
	// so the client workspace list can't see it and would offer an invalid "create dev" action.
	const devWorkspaceResource = resource(
		() => (currentIsRoot ? baseWorkspaceId : undefined),
		async (ws) => (ws ? await WorkspaceService.getDevWorkspace({ workspace: ws }) : undefined)
	)
	// Offer dev designation only once the server confirms there's no dev yet (returns null); stay
	// conservative (no offer) while the check is loading (current is undefined).
	let canDesignateDevWorkspace = $derived(currentIsRoot && devWorkspaceResource.current === null)
	let currentWorkspaceName = $derived(
		baseWorkspaceEntry?.name ?? baseWorkspaceId ?? 'the root workspace'
	)

	let id = $state('')
	let name = $state('')
	let username = $state('')

	let errorId = $state('')
	let errorUser = $state('')
	let aiKey = $state('')
	let codeCompletionEnabled = $state(true)
	let checking = $state(false)

	let forkDatatableSection: ReturnType<typeof ForkDatatableSection> | undefined = $state(undefined)
	let forkDucklakeSection: ReturnType<typeof ForkDucklakeSection> | undefined = $state(undefined)

	let workspaceColor: string | undefined = $state(undefined)
	let colorEnabled = $state(false)

	function generateRandomColor() {
		const randomColor =
			'#' +
			Math.floor(Math.random() * 16777215)
				.toString(16)
				.padStart(6, '0')
		workspaceColor = randomColor
	}

	async function validateName(id: string): Promise<void> {
		checking = true
		// For forks the actual workspace id is prefixed: checking the bare id
		// would report the name as free even when `wm-fork-<id>` is taken
		// (e.g. by an archived fork, which keeps its id reserved).
		const effectiveId = isFork && !createAsDevWorkspace ? `${WM_FORK_PREFIX}${id}` : id
		let exists =
			id != '' && (await WorkspaceService.existsWorkspace({ requestBody: { id: effectiveId } }))
		// The "delete existing fork to reclaim the id" affordance is only for prefixed forks.
		forkIdTaken = isFork && !createAsDevWorkspace && exists
		if (exists) {
			errorId = forkIdTaken
				? `A workspace with id '${effectiveId}' already exists. It may be an archived fork: archiving keeps the id reserved.`
				: 'ID already exists'
		} else if (id != '' && !/^\w+(-\w+)*$/.test(id)) {
			errorId = 'ID can only contain letters, numbers and dashes and must not finish by a dash'
		} else if (effectiveId.length > 50) {
			// `wm-fork-` prefix included: matches the backend's 50-char (git-branch / DB) limit.
			errorId = `ID '${effectiveId}' is too long (${effectiveId.length} chars). Maximum is 50.`
		} else {
			errorId = ''
		}
		checking = false
	}

	const WM_FORK_PREFIX = 'wm-fork-'

	// A dev workspace keeps its bare id; an ordinary fork is prefixed with `wm-fork-`.
	const effectiveForkId = $derived(createAsDevWorkspace ? id : `${WM_FORK_PREFIX}${id}`)

	let forkIdTaken = $state(false)
	let deleteExistingForkOpen = $state(false)
	let deletingExistingFork = $state(false)

	async function deleteExistingFork(): Promise<void> {
		if (deletingExistingFork) return
		const prefixedId = `${WM_FORK_PREFIX}${id}`
		deletingExistingFork = true
		try {
			await WorkspaceService.deleteWorkspace({ workspace: prefixedId })
			// Drop local sessions bound to this id so they don't resurface (or
			// auto-unarchive) against a new fork recreated under the same id.
			// Fire-and-forget: neither a slow nor a failing IndexedDB op should
			// block the delete/reuse flow (cleanup completes long before the UI
			// could create a session in a recreated fork).
			void deleteSessionsForWorkspace(prefixedId).catch((e) =>
				console.error(`Session cleanup for reused fork id ${prefixedId} failed`, e)
			)
			sendUserToast(`Permanently deleted workspace ${prefixedId}`)
			deleteExistingForkOpen = false
			await validateName(id)
		} catch (e: any) {
			sendUserToast(`Failed to delete workspace ${prefixedId}: ${e?.body?.toString() ?? e}`, true)
		} finally {
			deletingExistingFork = false
		}
	}

	let forkCreationLoading = $state(false)
	let forkCreationError = $state('')
	let errorMsgs: string[] = $state([])
	let failedSyncJobs: string[] = $state([])

	async function fetchFailedSyncJobs(jobs: string[]): Promise<CompletedJob[]> {
		let ret: CompletedJob[] = []
		for (const job of jobs) {
			let j = await JobService.getCompletedJob({
				id: job,
				workspace: baseWorkspaceId!
			})
			ret.push(j)
		}
		return ret
	}

	function isPathVersionLessThan(path: string | undefined, version: number): boolean {
		if (!path || !path.startsWith('hub/')) {
			return false
		}

		const parts = path.split('/')

		if (parts.length < 2) {
			return false
		}

		const embeddedVersion = parseInt(parts[1], 10)

		if (isNaN(embeddedVersion)) {
			return false
		}

		return embeddedVersion < version
	}

	async function createOrForkWorkspace() {
		if (isFork) {
			await forkWorkspace(effectiveForkId)
		} else {
			await createWorkspace()
		}
	}

	async function forkWorkspace(prefixed_id: string): Promise<void> {
		if (baseWorkspaceId) {
			forkCreationLoading = true
			errorMsgs = []
			failedSyncJobs = []
			forkCreationError = ''

			// Clone datatables BEFORE creating the workspace fork
			if (forkDatatableSection) {
				const queue = forkDatatableSection.buildCloneQueue(prefixed_id)
				if (queue.length > 0) {
					forkDatatableSection.startCloning(queue)
					return
				}
			}

			await completeFork(prefixed_id)
		} else {
			sendUserToast('No workspace selected, cannot fork non-existent workspace', true)
		}
	}

	async function completeFork(prefixed_id: string): Promise<void> {
		let gitSyncJobIds: string[]
		try {
			gitSyncJobIds = await WorkspaceService.createWorkspaceForkGitBranch({
				workspace: baseWorkspaceId!,
				requestBody: {
					id: prefixed_id,
					name,
					color: colorEnabled && workspaceColor ? workspaceColor : undefined,
					is_dev_workspace: createAsDevWorkspace,
					// Send the lock intent in this first phase too so the backend can reject a non-admin's
					// locked-dev request before any branch is created (avoids dangling branches).
					lock_prod_deploy: createAsDevWorkspace && lockProdDeploy,
					lock_prod_forking: createAsDevWorkspace && lockProdForking,
					copy_members: copyMembers
				}
			})
		} catch (e) {
			// The backend can reject here (fork cap, depth limit, premium, non-admin lock). Reset the
			// loading state and surface the error rather than leaving the button spinning.
			forkCreationError = `Failed to create fork '${prefixed_id}'`
			errorMsgs.push(e?.body ?? e ?? 'Unknown error')
			forkCreationLoading = false
			sendUserToast(`Could not create fork '${prefixed_id}' ${e?.body ?? e}`, true)
			return
		}

		try {
			await Promise.all(
				gitSyncJobIds.map((jobId) =>
					jobManager.runWithProgress(() => Promise.resolve(jobId), {
						workspace: baseWorkspaceId!,
						timeout: 60000,
						timeoutMessage: `Deploy fork job timed out after 60s`,
						onProgress: (status) => {
							if (status.status === 'failure') {
								errorMsgs.push(status.error ?? 'Deploy fork job failed')
								failedSyncJobs.push(jobId)
							}
						}
					})
				)
			)
		} catch (error) {
			forkCreationLoading = false
			sendUserToast(
				`Could not fork workspace ${baseWorkspaceId} because branch creation failed: ${errorMsgs} - ${error}`,
				true
			)
			return
		}
		if (errorMsgs.length != 0) {
			forkCreationError = 'Failed to create a branch for this fork on the git sync repo(s)'
			forkCreationLoading = false
			sendUserToast(
				`Could not fork workspace ${baseWorkspaceId} because branch creation failed: ${errorMsgs}`,
				true
			)
			return
		}

		// Build forked_datatables info from completed clone jobs
		const forkedDatatables = forkDatatableSection
			? forkDatatableSection.getCompletedCloneJobs().map((job) => ({
					name: job.name,
					new_dbname: job._newDbName
				}))
			: []

		try {
			await WorkspaceService.createWorkspaceFork({
				workspace: baseWorkspaceId!,
				requestBody: {
					id: prefixed_id,
					name,
					color: colorEnabled && workspaceColor ? workspaceColor : undefined,
					forked_datatables: forkedDatatables,
					shared_ducklakes: forkDucklakeSection?.getSharedDucklakes() ?? [],
					is_dev_workspace: createAsDevWorkspace,
					lock_prod_deploy: createAsDevWorkspace && lockProdDeploy,
					lock_prod_forking: createAsDevWorkspace && lockProdForking,
					copy_members: copyMembers
				}
			})
		} catch (e) {
			forkCreationError = `Failed to create fork '${prefixed_id}'`
			errorMsgs.push(e?.body ?? e ?? 'Unknown error')
			forkCreationLoading = false
			sendUserToast(`Could not create fork '${prefixed_id}' ${e}`, true)
			return
		}

		forkCreationLoading = false
		sendUserToast(
			createAsDevWorkspace
				? `Created dev workspace ${effectiveForkId} for ${baseWorkspaceId}`
				: `Successfully forked workspace ${baseWorkspaceId} as: wm-fork-${id}`
		)

		usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
		switchWorkspace(prefixed_id)

		onFinish?.()
	}

	async function createWorkspace(): Promise<void> {
		await WorkspaceService.createWorkspace({
			requestBody: {
				id,
				name,
				color: colorEnabled && workspaceColor ? workspaceColor : undefined,
				username: automateUsernameCreation ? undefined : username
			}
		})
		if (auto_invite) {
			await WorkspaceService.editAutoInvite({
				workspace: id,
				requestBody: { operator: operatorOnly, invite_all: !isCloudHosted(), auto_add: autoAdd }
			})
		}
		if (aiKey != '') {
			let actualUsername = username
			if (automateUsernameCreation) {
				const user = await UserService.whoami({
					workspace: id
				})
				actualUsername = user.username
			}
			let path = `u/${actualUsername}/${selected}_windmill_codegen`
			await VariableService.createVariable({
				workspace: id,
				requestBody: {
					path,
					value: aiKey,
					is_secret: true,
					description: 'Ai token'
				}
			})
			await ResourceService.createResource({
				workspace: id,
				requestBody: {
					path,
					value: {
						api_key: '$var:' + path
					},
					resource_type: selected
				}
			})
			await WorkspaceService.editCopilotConfig({
				workspace: id,
				requestBody: aiKey
					? {
							providers: {
								[selected]: {
									resource_path: path,
									models: [AI_PROVIDERS[selected].defaultModels[0]]
								}
							},
							default_model: {
								model: AI_PROVIDERS[selected].defaultModels[0],
								provider: selected
							},
							code_completion_model: codeCompletionEnabled
								? { model: AI_PROVIDERS[selected].defaultModels[0], provider: selected }
								: undefined
						}
					: {}
			})

			usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
			switchWorkspace(id)
		}

		sendUserToast(`Created workspace id: ${id}`)

		usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
		switchWorkspace(id)
		onFinish?.()
	}

	function handleKeyUp(event: KeyboardEvent) {
		const key = event.key
		if (key === 'Enter') {
			event.preventDefault()
			createWorkspace()
		}
	}

	async function loadWorkspaces() {
		if (!$usersWorkspaceStore) {
			try {
				usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
			} catch {}
		}
		if (!$usersWorkspaceStore) {
			const url = page.url
			console.log('logout 2')
			await logoutWithRedirect(url.href.replace(url.origin, ''))
		}
	}

	let automateUsernameCreation = $state(true)
	async function getAutomateUsernameCreationSetting() {
		automateUsernameCreation =
			((await SettingService.getGlobal({ key: 'automate_username_creation' })) as any) ?? true

		if (!automateUsernameCreation) {
			UserService.globalWhoami().then((x) => {
				let uname = ''
				if (x.name) {
					uname = x.name.split(' ')[0]
				} else {
					uname = x.email.split('@')[0]
				}
				uname = uname.replace(/\./gi, '')
				username = uname.toLowerCase()
			})
		}
	}
	getAutomateUsernameCreationSetting()

	onMount(() => {
		loadWorkspaces()

		WorkspaceService.isDomainAllowed().then((x) => {
			isDomainAllowed = x
		})
	})

	let isDomainAllowed: undefined | boolean = $state(undefined)

	let auto_invite = $state(false)
	let operatorOnly = $state(false)
	let autoAdd = $state(true)
	let selected: Exclude<AIProvider, 'customai'> = $state('openai')
	run(() => {
		if (isFork) {
			// Forks have no separate display name — the unique id is the name.
			name = id
		} else {
			id = name.toLowerCase().replace(/\s/gi, '-')
		}
	})
	run(() => {
		validateName(id)
	})
	run(() => {
		errorUser = validateUsername(username)
	})
	run(() => {
		colorEnabled && !workspaceColor && generateRandomColor()
	})
	let domain = $derived($usersWorkspaceStore?.email.split('@')[1])
</script>

<div class="flex flex-col flex-1 min-h-0">
	<!-- Standalone page: fixed 32rem scroll area (absolute inset). In the modal the
	     form flows naturally so the adaptive modal hugs the content, scrolling only
	     when capped by the viewport. -->
	<div class={inModal ? 'flex-1 min-h-0 overflow-y-auto' : 'flex-1 relative min-h-[32rem]'}>
		<div
			class={inModal
				? 'flex flex-col gap-8'
				: 'flex flex-col gap-8 absolute inset-0 overflow-y-auto'}
		>
			{#if errorMsgs.length != 0}
				<Alert class="p-2" title={forkCreationError} type="error">
					<ul class="pl-2 pr-4 break-words pb-5">
						{#each errorMsgs as errorMsg}
							<li><pre class="whitespace-pre-wrap">- {errorMsg}</pre></li>
						{/each}
					</ul>
					{#if failedSyncJobs.length != 0}
						More details on the jobs that failed:
						{#await fetchFailedSyncJobs(failedSyncJobs)}
							<LoaderCircle class="animate-spin" />
						{:then failedJobs}
							<ul class="pl-2 pr-4 break-words">
								{#each failedJobs as job}
									<li>
										-
										<a
											target="_blank"
											class="underline"
											href={`/run/${job.id}?workspace=${baseWorkspaceId}`}
										>
											{job.id}
										</a>
									</li>
									<!-- This 28073 is the version where git sync on fork was introduced -->
									{#if isPathVersionLessThan(job.script_path, 28073)}
										<div class="font-bold">
											This job was not running the latest version of the git sync script available
											on the hub. You might be able to solve this issue by going to `Workspace
											Settings` -> `Git Sync` and updating the script.
										</div>
									{/if}
								{/each}
							</ul>
						{:catch error}
							Tried to fetch jobs to get more information, but failed: {error}. Here are the failed
							job ids:
							<ul class="pl-2 pr-4 break-words">
								{#each failedSyncJobs as jobId}
									<li>
										-
										<a
											target="_blank"
											class="underline"
											href={`/run/${jobId}?workspace=${baseWorkspaceId}`}
										>
											{jobId}
										</a>
									</li>
								{/each}
							</ul>
						{/await}
					{/if}
				</Alert>
			{/if}
			{#if !isFork}
				<label class="flex flex-col gap-1">
					<span class="text-xs font-semibold text-emphasis">Workspace name</span>
					<span class="text-xs text-secondary">Displayable name</span>
					<!-- svelte-ignore a11y_autofocus -->
					<TextInput inputProps={{ autofocus: true }} bind:value={name} />
				</label>
			{/if}
			<label class="flex flex-col gap-1">
				{#if isFork}
					<span class="text-xs font-semibold text-emphasis">Fork ID</span>
					<span class="text-xs text-secondary"
						>Unique name of your fork (this will also set the branch name)</span
					>
				{:else}
					<span class="text-xs font-semibold text-emphasis">Workspace ID</span>
					<span class="text-xs text-secondary">Slug to uniquely identify your workspace</span>
				{/if}

				{#if isFork && !createAsDevWorkspace}
					<PrefixedInput
						prefix={WM_FORK_PREFIX}
						bind:value={id}
						placeholder="my-fork"
						autofocus
						error={errorId != ''}
					/>
				{:else}
					<TextInput bind:value={id} error={errorId} />
				{/if}
				{#if errorId}
					<span class="text-red-500 text-2xs font-normal">{errorId}</span>
					{#if forkIdTaken}
						<div class="flex flex-row items-center gap-2 pt-1">
							<Button
								destructive
								size="xs"
								startIcon={{ icon: Trash2 }}
								disabled={deletingExistingFork}
								on:click={() => (deleteExistingForkOpen = true)}
							>
								Permanently delete existing fork
							</Button>
							<span class="text-2xs text-secondary">
								Frees up the id so you can reuse it (fork owner or superadmin only)
							</span>
						</div>
					{/if}
				{/if}
			</label>
			<Label label={isFork ? 'Fork color' : 'Workspace color'}>
				{#snippet header()}
					<span class="text-2xs text-secondary">(optional)</span>
				{/snippet}
				<span class="text-xs text-secondary">
					Color to identify the current workspace in the list of workspaces
				</span>
				<div class="flex items-center gap-4">
					<Toggle bind:checked={colorEnabled} options={{ right: 'Enable' }} />
					{#if colorEnabled}
						<div class="flex items-center gap-1 grow">
							<input
								class="grow min-w-10"
								type="color"
								bind:value={workspaceColor}
								disabled={!colorEnabled}
							/>

							<TextInput
								class="w-24"
								bind:value={workspaceColor}
								inputProps={{ disabled: !colorEnabled }}
							/>
							<Button
								on:click={generateRandomColor}
								size="xs"
								variant="default"
								disabled={!colorEnabled}>Random</Button
							>
						</div>
					{/if}
				</div>
				{#if isFork && colorEnabled}
					<div class="flex flex-col gap-1 pt-2">
						<div class="flex flex-row items-center gap-1">
							<span class="text-2xs text-secondary">Fork picker preview</span>
							<!-- Toggles the real app theme: a preview-local .dark scope can't
							     re-theme the modal (theme vars are bound to html.dark), and the
							     actual theme is what the chip must be judged against anyway. -->
							<DarkModeToggle forcedDarkMode={false} />
						</div>
						<div class="w-64">
							<WorkspaceScopeTrigger
								workspaceId={baseWorkspaceId}
								pendingFork={{
									id: effectiveForkId,
									name: id || 'my-fork',
									parent_workspace_id: baseWorkspaceId ?? ''
								}}
								color={workspaceColor}
								showChevron={false}
								wrap
								class="w-full pointer-events-none"
							/>
						</div>
					</div>
				{/if}
			</Label>
			{#if isFork && canDesignateDevWorkspace}
				<Label label="Persistent dev workspace">
					<span class="text-xs text-secondary">
						Create a standing dev workspace (no <code>wm-fork-</code> prefix) paired with this workspace,
						instead of a throwaway fork.
					</span>
					<div class="flex flex-col gap-2 pt-1">
						<Toggle bind:checked={createAsDevWorkspace} options={{ right: 'Dev workspace' }} />
						{#if createAsDevWorkspace}
							<div class="flex flex-col gap-2 rounded-md border bg-surface-secondary p-3">
								<div class="flex flex-col gap-0.5">
									<span class="text-xs font-semibold text-emphasis"
										>Protect {currentWorkspaceName}</span
									>
									<span class="text-2xs text-secondary">
										Adds protection rules to this (root) workspace so changes are made in the new
										dev workspace and promoted here.
									</span>
								</div>
								<Toggle
									bind:checked={lockProdDeploy}
									options={{ right: 'Block direct edits (deploy via the dev workspace)' }}
								/>
								<Toggle bind:checked={lockProdForking} options={{ right: 'Prevent forking' }} />
							</div>
						{/if}
					</div>
				</Label>
			{/if}
			{#if isFork && createAsDevWorkspace}
				<Label label="Members">
					<span class="text-xs text-secondary">
						Copy this workspace's members (and their group memberships) into the dev workspace so
						the team can work in it.
					</span>
					<div class="pt-1">
						<Toggle bind:checked={copyMembers} options={{ right: 'Copy members' }} />
					</div>
				</Label>
			{/if}
			{#if isFork}
				<label class="flex flex-col gap-1">
					<div class="flex flex-row gap-2 items-baseline">
						<span class="text-xs font-semibold text-emphasis">Base workspace</span>
						<span class="text-2xs text-secondary">(optional)</span>
					</div>
					<span class="text-xs text-secondary">
						{#if createAsDevWorkspace}
							A dev workspace is always based on the root workspace.
						{:else}
							Workspace to fork from. Defaults to the root; pick an existing fork to create a fork
							of a fork (the new branch is based on the selected workspace's branch).
						{/if}
					</span>
					<Select
						items={baseItems}
						bind:value={baseWorkspaceId}
						placeholder="Select a base workspace"
						disabled={createAsDevWorkspace}
					/>
				</label>
			{/if}
			{#if isFork}
				<ForkDatatableSection
					bind:this={forkDatatableSection}
					sourceWorkspace={baseWorkspaceId}
					onAllDone={() => {
						completeFork(effectiveForkId)
					}}
					onCanceled={() => {
						forkCreationLoading = false
					}}
				/>
				<ForkDucklakeSection bind:this={forkDucklakeSection} />
			{/if}
			{#if !automateUsernameCreation}
				<Label label="Your username in that workspace">
					<TextInput
						bind:value={username}
						inputProps={{ onkeyup: handleKeyUp }}
						error={errorUser}
					/>
					{#if errorUser}
						<span class="text-red-500 text-2xs">{errorUser}</span>
					{/if}
				</Label>
			{/if}
			{#if !isFork}
				<div class="block">
					<div class="flex flex-col gap-1">
						<label for="ai-key" class="flex flex-row gap-2">
							<span class="text-xs font-semibold text-emphasis">
								AI key for Windmill AI
								<Tooltip>
									Find out how it can help you <a
										href="https://www.windmill.dev/docs/core_concepts/ai_generation"
										target="_blank"
										rel="noopener noreferrer">in the docs</a
									>
								</Tooltip>
							</span>
							<span class="text-2xs text-secondary">(optional but recommended)</span>
						</label>

						<ToggleButtonGroup bind:selected>
							{#snippet children({ item })}
								<ToggleButton value="openai" label="OpenAI" {item} />
								<ToggleButton value="anthropic" label="Anthropic" {item} />
								<ToggleButton value="mistral" label="Mistral" {item} />
								<ToggleButton value="deepseek" label="DeepSeek" {item} />
							{/snippet}
						</ToggleButtonGroup>
						<div class="flex flex-row gap-1">
							<input
								id="ai-key"
								type="password"
								autocomplete="new-password"
								bind:value={aiKey}
								onkeyup={handleKeyUp}
							/>
							<TestAIKey
								apiKey={aiKey}
								disabled={!aiKey}
								aiProvider={selected}
								model={AI_PROVIDERS[selected].defaultModels[0]}
							/>
						</div>
					</div>

					{#if aiKey}
						<div class="flex flex-col gap-2 mt-2">
							<Toggle
								disabled={!aiKey}
								bind:checked={codeCompletionEnabled}
								options={{ right: 'Enable code completion' }}
							/>
						</div>
					{/if}
				</div>
				<div class="flex flex-col gap-1">
					<label for="auto-invite" class="text-xs font-semibold text-emphasis"
						>{isCloudHosted()
							? `Auto-${autoAdd ? 'add' : 'invite'} anyone from ${domain}`
							: `Auto-${autoAdd ? 'add' : 'invite'} anyone joining the instance`}</label
					>
					<Toggle
						id="auto-invite"
						disabled={isCloudHosted() && !isDomainAllowed}
						bind:checked={auto_invite}
					/>
					{#if isCloudHosted() && isDomainAllowed == false}
						<div class="text-secondary text-2xs">{domain} domain not allowed for auto-invite</div>
					{/if}

					{#if auto_invite}
						<div class="bg-surface-tertiary p-4 rounded-md flex flex-col gap-8">
							<!-- svelte-ignore a11y_label_has_associated_control -->
							{#if isCloudHosted()}
								<label class="flex flex-col gap-1">
									<span class="text-xs font-semibold text-emphasis">Mode</span>
									<span class="text-xs text-secondary font-normal"
										>Whether to invite or add users directly to the workspace.</span
									>
									<ToggleButtonGroup
										selected={autoAdd ? 'add' : 'invite'}
										on:selected={async (e) => {
											autoAdd = e.detail === 'add'
										}}
									>
										{#snippet children({ item })}
											<ToggleButton value="invite" label="Auto-invite" {item} />
											<ToggleButton value="add" label="Auto-add" {item} />
										{/snippet}
									</ToggleButtonGroup>
								</label>
							{/if}

							<label class="font-semibold flex flex-col gap-1">
								<span class="text-xs font-semibold text-emphasis">Role</span>
								<span class="text-xs text-secondary font-normal"
									>Role of the auto-invited users</span
								>
								<ToggleButtonGroup
									selected={operatorOnly ? 'operator' : 'developer'}
									on:selected={(e) => {
										operatorOnly = e.detail == 'operator'
									}}
								>
									{#snippet children({ item })}
										<ToggleButton value="operator" label="Operator" {item} />
										<ToggleButton value="developer" label="Developer" {item} />
									{/snippet}
								</ToggleButtonGroup>
							</label>
						</div>
					{/if}
				</div>
			{/if}
		</div>
	</div>
	<div class="flex flex-wrap flex-row {inModal ? 'justify-end' : 'justify-between'} gap-4 pt-4">
		{#if !inModal}
			<Button
				disabled={forkCreationLoading}
				variant="default"
				size="sm"
				href="{base}/user/workspaces">&leftarrow; Back to workspaces</Button
			>
		{/if}
		{#if !forkCreationLoading}
			<Button
				variant="accent"
				disabled={checking ||
					errorId != '' ||
					!name ||
					(!automateUsernameCreation && (errorUser != '' || !username)) ||
					!id}
				on:click={createOrForkWorkspace}
			>
				{#if isFork}
					Fork workspace
				{:else}
					Create workspace
				{/if}
			</Button>
		{:else}
			<Button variant="accent" disabled={true}>
				<LoaderCircle class="animate-spin" /> Creating branch
			</Button>
		{/if}
	</div>
</div>

<ConfirmationModal
	open={deleteExistingForkOpen}
	title="Permanently delete existing fork"
	confirmationText="Delete permanently"
	loading={deletingExistingFork}
	on:canceled={() => {
		deleteExistingForkOpen = false
	}}
	on:confirmed={() => {
		deleteExistingFork()
	}}
>
	<div class="flex flex-col w-full space-y-4">
		<span>
			This will permanently delete the workspace '{WM_FORK_PREFIX}{id}' and all of its content
			(scripts, flows, apps, variables, resources, runs). This cannot be undone. Unlike archiving,
			this frees up the workspace id for a new fork.
		</span>
	</div>
</ConfirmationModal>
