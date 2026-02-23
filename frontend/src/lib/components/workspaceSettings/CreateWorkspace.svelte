<script lang="ts">
	import { run } from 'svelte/legacy'

	import { goto } from '$lib/navigation'
	import { base } from '$lib/base'
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
	import { page } from '$app/stores'
	import { usersWorkspaceStore, workspaceStore } from '$lib/stores'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Button } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { onMount } from 'svelte'
	import { sendUserToast } from '$lib/toast'
	import TestAIKey from '$lib/components/copilot/TestAIKey.svelte'
	import { switchWorkspace } from '$lib/storeUtils'
	import { isCloudHosted } from '$lib/cloud'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { AI_PROVIDERS } from '$lib/components/copilot/lib'
	import { GitFork, LoaderCircle } from 'lucide-svelte'
	import PrefixedInput from '../PrefixedInput.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import { jobManager } from '$lib/services/JobManager'
	import Alert from '../common/alert/Alert.svelte'

	interface Props {
		isFork?: boolean
	}

	let { isFork = false }: Props = $props()

	const rd = $page.url.searchParams.get('rd')

	let id = $state('')
	let name = $state('')
	let username = $state('')

	let errorId = $state('')
	let errorUser = $state('')
	let aiKey = $state('')
	let codeCompletionEnabled = $state(true)
	let checking = $state(false)

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
		let exists = await WorkspaceService.existsWorkspace({ requestBody: { id } })
		if (exists) {
			errorId = 'ID already exists'
		} else if (id != '' && !/^\w+(-\w+)*$/.test(id)) {
			errorId = 'ID can only contain letters, numbers and dashes and must not finish by a dash'
		} else {
			errorId = ''
		}
		checking = false
	}

	const WM_FORK_PREFIX = 'wm-fork-'

	let forkCreationLoading = $state(false)
	let forkCreationError = $state('')
	let errorMsgs: string[] = $state([])
	let failedSyncJobs: string[] = $state([])

	async function fetchFailedSyncJobs(jobs: string[]): Promise<CompletedJob[]> {
		let ret: CompletedJob[] = []
		for (const job of jobs) {
			let j = await JobService.getCompletedJob({
				id: job,
				workspace: $workspaceStore!
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
		const prefixed_id = `${WM_FORK_PREFIX}${id}`
		if (isFork) {
			if ($workspaceStore) {
				forkCreationLoading = true
				errorMsgs = []
				failedSyncJobs = []
				forkCreationError = ''

				let gitSyncJobIds = await WorkspaceService.createWorkspaceForkGitBranch({
					workspace: $workspaceStore!,
					requestBody: {
						id: prefixed_id,
						name,
						color: colorEnabled && workspaceColor ? workspaceColor : undefined
					}
				})

				try {
					await Promise.all(
						gitSyncJobIds.map((jobId) =>
							jobManager.runWithProgress(() => Promise.resolve(jobId), {
								workspace: $workspaceStore,
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
						`Could not fork workspace ${$workspaceStore} because branch creation failed: ${errorMsgs} - ${error}`,
						true
					)
					return
				}
				if (errorMsgs.length != 0) {
					forkCreationError = 'Failed to create a branch for this fork on the git sync repo(s)'
					forkCreationLoading = false
					sendUserToast(
						`Could not fork workspace ${$workspaceStore} because branch creation failed: ${errorMsgs}`,
						true
					)
					return
				}

				try {
					await WorkspaceService.createWorkspaceFork({
						workspace: $workspaceStore!,
						requestBody: {
							id: prefixed_id,
							name,
							color: colorEnabled && workspaceColor ? workspaceColor : undefined
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
				sendUserToast(`Successfully forked workspace ${$workspaceStore} as: wm-fork-${id}`)
			} else {
				sendUserToast('No workspace selected, cannot fork non-existent workspace', true)
			}
		} else {
			await createWorkspace()
		}

		usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
		switchWorkspace(isFork ? prefixed_id : id)

		goto(rd ?? '/')
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
		}

		sendUserToast(`Created workspace id: ${id}`)

		usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
		switchWorkspace(id)

		goto(rd ?? '/')
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
			const url = $page.url
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
		id = name.toLowerCase().replace(/\s/gi, '-')
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

<CenteredModal title="{isFork ? 'Forking' : 'New'} Workspace" centerVertically={false}>
	<div class="flex flex-col gap-8">
		{#if isFork}
			<div class="flex flex-block gap-2">
				<GitFork size={16} />
				<span class="text-xs text-normal">Forking </span>
				<span class="text-xs text-emphasis font-semibold">
					{$workspaceStore}
				</span>
			</div>
		{/if}
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
										href={`/run/${job.id}?workspace=${$workspaceStore}`}
									>
										{job.id}
									</a>
								</li>
								<!-- This 28073 is the version where git sync on fork was introduced -->
								{#if isPathVersionLessThan(job.script_path, 28073)}
									<div class="font-bold">
										This job was not running the latest version of the git sync script available on
										the hub. You might be able to solve this issue by going to `Workspace Settings`
										-> `Git Sync` and updating the script.
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
										href={`/run/${jobId}?workspace=${$workspaceStore}`}
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
		<label class="flex flex-col gap-1">
			{#if isFork}
				<span class="text-xs font-semibold text-emphasis">Fork name</span>
				<span class="text-xs text-secondary">Displayable name of the forked workspace</span>
			{:else}
				<span class="text-xs font-semibold text-emphasis">Workspace name</span>
				<span class="text-xs text-secondary">Displayable name</span>
			{/if}
			<!-- svelte-ignore a11y_autofocus -->
			<TextInput inputProps={{ autofocus: true }} bind:value={name} />
		</label>
		<label class="flex flex-col gap-1">
			<span class="text-xs font-semibold text-emphasis">Workspace ID</span>
			{#if isFork}
				<span class="text-xs text-secondary"
					>Slug to uniquely identify your fork (this will also set the branch name)</span
				>
			{:else}
				<span class="text-xs text-secondary">Slug to uniquely identify your workspace</span>
			{/if}

			{#if isFork}
				<PrefixedInput
					prefix={WM_FORK_PREFIX}
					type="text"
					bind:value={id}
					placeholder="example.com"
					class={errorId != '' ? 'input-error' : ''}
				/>
			{:else}
				<TextInput bind:value={id} error={errorId} />
			{/if}
			{#if errorId}
				<span class="text-red-500 text-2xs font-normal">{errorId}</span>
			{/if}
		</label>
		<label class="flex flex-col gap-1">
			<span class="text-xs font-semibold text-emphasis">Workspace color</span>
			<span class="text-xs text-secondary"
				>Color to identify the current workspace in the list of workspaces</span
			>
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
		</label>
		{#if !automateUsernameCreation}
			<label class="flex flex-col gap-1">
				<span class="text-xs font-semibold text-emphasis">Your username in that workspace</span>
				<TextInput bind:value={username} inputProps={{ onkeyup: handleKeyUp }} error={errorUser} />
				{#if errorUser}
					<span class="text-red-500 text-2xs">{errorUser}</span>
				{/if}
			</label>
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
							<span class="text-xs text-secondary font-normal">Role of the auto-invited users</span>
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
		<div class="flex flex-wrap flex-row justify-between gap-4 pt-4">
			<Button
				disabled={forkCreationLoading}
				variant="default"
				size="sm"
				href="{base}/user/workspaces">&leftarrow; Back to workspaces</Button
			>
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
</CenteredModal>
