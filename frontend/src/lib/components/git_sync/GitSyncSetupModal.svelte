<script lang="ts">
	import type { Snippet } from 'svelte'
	import { untrack } from 'svelte'
	import { ArrowRight, Database } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import Alert from '../common/alert/Alert.svelte'
	import Modal2 from '../common/modal/Modal2.svelte'
	import Stepper from '../common/stepper/Stepper.svelte'
	import ResourcePicker from '../ResourcePicker.svelte'
	import Path from '../Path.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import EEOnly from '../EEOnly.svelte'
	import RadioCard from '../common/radioCard/RadioCard.svelte'
	import GithubIcon from '../icons/GithubIcon.svelte'
	import GitlabIcon from '../icons/GitlabIcon.svelte'
	import GitlabProjectConnectForm from './setup/GitlabProjectConnectForm.svelte'
	import GithubAppConnectForm from './setup/GithubAppConnectForm.svelte'
	import TokenUrlConnectForm from './setup/TokenUrlConnectForm.svelte'
	import { createRepositoryResource, repoSlug, type RepoConnection } from './setup/repoConnection'
	import { GithubAppSetup } from './setup/githubAppSetup.svelte'
	import { onDestroy } from 'svelte'
	import { getGitSyncContext } from './GitSyncContext.svelte'
	import ConfigureSyncStep from './setup/ConfigureSyncStep.svelte'
	import GitPushPreview from './GitPushPreview.svelte'
	import GitSyncSuccessContent from './GitSyncSuccessContent.svelte'
	import { ResourceService } from '$lib/gen'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { apiErrorMessage } from '$lib/utils'

	type Provider = 'gitlab' | 'github_app' | 'github_pat' | 'existing'

	type Props = {
		opened: boolean
		title: string
		mode: 'sync' | 'promotion'
		/** Resources already attached to a repository of this workspace, which cannot be picked twice. */
		usedResourcePaths: string[]
	}

	let { opened = $bindable(), title, mode, usedResourcePaths }: Props = $props()

	const ctx = getGitSyncContext()

	const STEPS = [
		'Choose a provider',
		'Connect the repository',
		'Configure sync',
		'Initialize the repository',
		'Done'
	]

	let step: 1 | 2 | 3 | 4 | 5 = $state(1)
	/** What the last step reports, set by the save that got there. */
	let done: { savedWithoutInit: boolean; autoPullOn: boolean } | undefined = $state(undefined)

	/** Resource path of the unsaved repository step 3 configures. By path rather than
	 * index, as the context's list can shift under it. */
	let draftPath: string | undefined = $state(undefined)
	const draftIdx = $derived(
		draftPath === undefined
			? -1
			: ctx.repositories.findIndex((r) => r.git_repo_resource_path === draftPath)
	)
	const draft = $derived(draftIdx === -1 ? undefined : ctx.repositories[draftIdx])
	let push: GitPushPreview | undefined = $state()
	/** Step 3 has read what Windmill holds for the repository. */
	let factsLoaded = $state(false)
	let saving = $state(false)
	let saveError: string | undefined = $state(undefined)
	let provider: Provider | undefined = $state(undefined)
	let existingPath: string | undefined = $state(undefined)
	/** The workspace has a git_repository resource no repository uses yet, so picking one is
	 *  offered up front rather than behind the hint. */
	let hasResource = $state(false)

	let connection: RepoConnection | undefined = $state(undefined)
	/** Path of the resource the run creates. `Path` keeps whatever non-empty path it is bound
	 * to, so it has to be cleared when the repository changes — otherwise the second
	 * repository is offered the first one's path, which is already taken. */
	let newPath = $state('')
	let newPathError = $state('')
	let branch = $state('')
	let folder = $state('')
	let connecting = $state(false)
	let connectError: string | undefined = $state(undefined)
	let githubApp: GithubAppSetup | undefined = $state(undefined)

	/** A push started from step 4. Owned here rather than read off `GitPushPreview`: closing
	 *  the dialog destroys that component in the same flush as the dismissal, so its state is
	 *  already gone by the time the dismissal is handled. */
	let applying = $state(false)
	/** Work in flight that a dismissal must not cut short. */
	const busy = $derived(connecting || saving || applying)

	onDestroy(() => {
		githubApp?.dispose()
		discardDraft()
	})
	$effect(() => {
		if (!opened)
			untrack(() => {
				githubApp?.dispose()
				discardDraft()
			})
	})

	/** Drops the unsaved repository, keeping the resource, which stays pickable.
	 *
	 * A push that is still running is left alone: it may yet initialize the repository, and
	 * the row it leaves behind is how the user finishes saving the connection. */
	function discardDraft() {
		if (busy) {
			draftPath = undefined
			return
		}
		if (draftIdx !== -1 && ctx.repositories[draftIdx]?.isUnsavedConnection) {
			void ctx.removeRepository(draftIdx)
		}
		draftPath = undefined
	}

	function enterConfigure(path: string) {
		discardDraft()
		factsLoaded = false
		const before = ctx.repositories.length
		if (mode === 'promotion') ctx.addPromotionRepository()
		else ctx.addSyncRepository()
		// Refused (CE is limited to one repository): the context toasted why.
		if (ctx.repositories.length === before) return
		ctx.repositories[before].git_repo_resource_path = path
		draftPath = path
		saveError = undefined
		step = 3
	}

	/** From step 3 the resource already exists, so going back offers it as the
	 * existing resource to use rather than reconnecting from scratch. */
	function back() {
		if (busy) return
		if (step === 4) {
			step = 3
			return
		}
		if (step === 3 && draftPath) {
			const path = draftPath
			discardDraft()
			provider = 'existing'
			existingPath = path
		}
		step = 1
	}

	async function saveDraft(withoutInit = false) {
		if (draftIdx === -1) return
		saving = true
		saveError = undefined
		try {
			// The last step says what the success modal would have, so it stays quiet.
			await ctx.saveRepository(draftIdx, withoutInit, false)
			done = {
				savedWithoutInit: withoutInit,
				autoPullOn: draft?.auto_pull?.enabled === true
			}
			step = 5
		} catch (e) {
			saveError = apiErrorMessage(e)
		} finally {
			saving = false
		}
	}

	async function reset() {
		step = 1
		done = undefined
		applying = false
		githubApp?.dispose()
		githubApp = undefined
		hasResource = false
		provider = undefined
		existingPath = undefined
		connection = undefined
		newPath = ''
		newPathError = ''
		namedFor = undefined
		branch = ''
		folder = ''
		connectError = undefined
		const workspace = $workspaceStore
		if (!workspace) return
		try {
			const resources = await ResourceService.listResource({
				workspace,
				resourceType: 'git_repository'
			})
			const firstUnused = resources.find((r) => !usedResourcePaths.includes(r.path))
			hasResource = !!firstUnused
			if (provider === undefined && firstUnused) {
				provider = 'existing'
				existingPath = firstUnused.path
			}
		} catch (e) {
			console.error('Failed to list git_repository resources', e)
		}
	}

	$effect(() => {
		if (opened) untrack(() => reset())
	})

	// Cast: the only writes to `connection` are the `bind:` on the provider forms, which the
	// checker does not see as assignments — it narrows the variable to `undefined` without it.
	const repoName = $derived.by(() => {
		const url = (connection as RepoConnection | undefined)?.url
		return url ? repoSlug(url) : undefined
	})
	let namedFor: string | undefined = $state(undefined)
	$effect(() => {
		const name = repoName
		untrack(() => {
			if (name === namedFor) return
			namedFor = name
			newPath = ''
			newPathError = ''
		})
	})

	// GitLab without EE has no server-side token store nor project listing, so it
	// takes the same URL + token form as a GitHub PAT.
	const gitlabHeld = $derived(provider === 'gitlab' && !!$enterpriseLicense)

	const canContinue = $derived(
		step === 1
			? provider === 'existing'
				? !!existingPath
				: provider !== undefined
			: step === 2
				? !!connection && !!newPath && !newPathError && !connecting
				: (draft?.detectionState === 'no-wmill' || draft?.detectionState === 'has-wmill') &&
					factsLoaded &&
					!saving
	)

	// Steps this run skips: connecting, when an existing resource is used, and initializing,
	// when the repository already holds a Windmill configuration.
	const disabledIndices = $derived([
		...(provider === 'existing' ? [1] : []),
		...(draft?.detectionState === 'has-wmill' ? [3] : [])
	])

	function selectProvider(p: Provider) {
		if (p !== provider) connection = undefined
		provider = p
		if (p === 'github_app' && !githubApp && $workspaceStore) {
			githubApp = new GithubAppSetup($workspaceStore)
			void githubApp.reload()
		}
	}

	async function next() {
		if (step === 1) {
			if (provider === 'existing') {
				if (existingPath) enterConfigure(existingPath)
			} else {
				connectError = undefined
				step = 2
				// Nothing to pick from yet: go straight to installing. Only while this click
				// is still being handled, or the browser blocks the tab.
				if (provider === 'github_app' && githubApp?.loaded && githubApp.usable.length === 0) {
					githubApp.openInstall()
				}
			}
			return
		}
		const workspace = $workspaceStore
		if (!connection || !workspace) return
		connecting = true
		connectError = undefined
		// Read once: the fields stay editable while the request is in flight, and reading the
		// path again afterwards would configure a repository this run never created.
		const submitted = { path: newPath, connection, branch, folder }
		try {
			await createRepositoryResource(workspace, submitted.path, submitted.connection, {
				branch: submitted.branch,
				folder: submitted.folder
			})
			enterConfigure(submitted.path)
		} catch (e) {
			connectError = apiErrorMessage(e)
		} finally {
			connecting = false
		}
	}
</script>

<Modal2
	bind:isOpen={opened}
	target="#content"
	formStyling
	preventDismiss={busy}
	{title}
	contentClasses="flex flex-col"
	fixedWidth="md"
	fixedHeight="lg"
>
	<div class="flex h-full flex-col gap-4">
		<Stepper
			tabs={STEPS}
			selectedIndex={step - 1}
			maxReachedIndex={step - 1}
			{disabledIndices}
			small
			on:click={(e) => {
				if (e.detail.index === 0 && step > 1) back()
			}}
		/>

		<div class="flex-1 flex flex-col min-h-0">
			<div class="flex-1 overflow-y-auto flex flex-col gap-2">
				{#if step === 1}
					{#snippet githubIcon()}
						<GithubIcon height={18} width={18} />
					{/snippet}
					{#snippet gitlabIcon()}
						<GitlabIcon height={18} width={18} />
					{/snippet}
					{#snippet resourceIcon()}
						<Database size={18} class="text-secondary" />
					{/snippet}
					<div
						role="radiogroup"
						aria-label="How to connect the repository"
						class="flex flex-col gap-2"
					>
						{@render providerCard(
							'github_app',
							githubIcon,
							'GitHub App (recommended)',
							'Install the Windmill GitHub App on your repository. Enables webhooks, pull requests and commit checks.',
							!$enterpriseLicense
						)}
						{@render providerCard(
							'github_pat',
							githubIcon,
							'GitHub personal access token',
							'Connect a GitHub repository with a fine-grained personal access token.'
						)}
						{@render providerCard(
							'gitlab',
							gitlabIcon,
							'GitLab',
							'Connect a GitLab repository with an access token.'
						)}
						{#if hasResource || provider === 'existing'}
							{@render providerCard(
								'existing',
								resourceIcon,
								'Use an existing git_repository resource',
								'Pick a resource that already holds the repository URL and its credentials.'
							)}
						{/if}
					</div>
					{#if provider === 'existing'}
						<ResourcePicker
							bind:value={existingPath}
							resourceType="git_repository"
							excludedValues={usedResourcePaths}
						/>
					{/if}
				{:else if step === 5}
					{#if done}
						<GitSyncSuccessContent
							savedWithoutInit={done.savedWithoutInit}
							autoPullOn={done.autoPullOn}
						/>
					{/if}
				{:else if step === 4}
					{#if draft}
						<GitPushPreview
							bind:this={push}
							gitRepoResourcePath={draft.git_repo_resource_path}
							uiState={draft.settings}
							onApplyStateChange={(v) => (applying = v)}
							onSuccess={() => void saveDraft()}
						/>
					{/if}
					{#if saveError}
						<Alert type="error" size="xs" title="Could not save the repository">
							{saveError}
						</Alert>
					{/if}
				{:else if step === 3}
					{#if draftIdx !== -1}
						{#key draftPath}
							<ConfigureSyncStep idx={draftIdx} {mode} bind:factsLoaded />
						{/key}
					{/if}
					{#if saveError}
						<Alert type="error" size="xs" title="Could not save the repository">
							{saveError}
						</Alert>
					{/if}
				{:else if $workspaceStore}
					<div class="flex flex-col gap-6">
						{#if provider === 'github_app' && githubApp}
							<GithubAppConnectForm workspace={$workspaceStore} setup={githubApp} bind:connection />
						{:else if gitlabHeld}
							<GitlabProjectConnectForm workspace={$workspaceStore} bind:connection />
						{:else if provider === 'gitlab'}
							<TokenUrlConnectForm provider="gitlab" bind:connection />
						{:else}
							<TokenUrlConnectForm provider="github" bind:connection />
						{/if}

						{#if connection}
							<div class="flex flex-col gap-4 border-t pt-4">
								<label class="flex flex-col gap-1">
									<span class="text-xs font-semibold text-emphasis">Branch</span>
									<span class="text-xs text-secondary">
										The branch deploys are committed to. Leave empty for the repository's default
										branch.
									</span>
									<TextInput bind:value={branch} inputProps={{ placeholder: 'Default branch' }} />
								</label>
								<label class="flex flex-col gap-1">
									<span class="text-xs font-semibold text-emphasis">Folder</span>
									<span class="text-xs text-secondary">
										Where in the repository the workspace is kept, relative to its root. Leave empty
										for the root. The folder must already exist in the repository.
									</span>
									<TextInput bind:value={folder} inputProps={{ placeholder: 'Repository root' }} />
								</label>
								<div class="flex flex-col gap-1">
									<span class="text-xs font-semibold text-emphasis">Save as resource</span>
									<span class="text-xs text-secondary">
										Windmill keeps the repository and its access as a <code>git_repository</code> resource
										at this path.
									</span>
									{#key connection.url}
										<Path
											kind="resource"
											initialPath=""
											fullNamePlaceholder={repoSlug(connection.url)}
											autofocus={false}
											bind:path={newPath}
											bind:error={newPathError}
										/>
									{/key}
								</div>
							</div>
						{/if}

						{#if connectError}
							<Alert type="error" size="xs" title="Could not connect the repository">
								{connectError}
							</Alert>
						{/if}
					</div>
				{/if}
			</div>

			<div class="flex justify-between items-center pt-3">
				<div>
					{#if step > 1 && step < 5}
						<Button unifiedSize="sm" variant="default" disabled={busy} onClick={back}>Back</Button>
					{:else if step === 1 && !hasResource && provider !== 'existing'}
						<Button variant="subtle" unifiedSize="sm" onClick={() => selectProvider('existing')}>
							Use an existing resource instead
						</Button>
					{/if}
				</div>
				{#if step === 5}
					<Button unifiedSize="sm" variant="accent" onClick={() => (opened = false)}>Close</Button>
				{:else if step === 4}
					<div class="flex items-center gap-2">
						<Button
							unifiedSize="sm"
							variant="default"
							disabled={push?.status().applying || push?.status().previewing || saving}
							loading={saving}
							onClick={() => saveDraft(true)}
						>
							Save without initializing
						</Button>
						<Button
							unifiedSize="sm"
							variant="accent"
							disabled={!push?.status().canApply || saving}
							loading={push?.status().applying}
							onClick={() => push?.apply()}
						>
							Initialize repository
						</Button>
					</div>
				{:else if step === 3}
					{#if draft?.detectionState === 'no-wmill'}
						<Button
							unifiedSize="sm"
							variant="accent"
							disabled={!canContinue}
							endIcon={{ icon: ArrowRight }}
							onClick={() => (step = 4)}
						>
							Next
						</Button>
					{:else}
						<Button
							unifiedSize="sm"
							variant="accent"
							disabled={!canContinue}
							loading={saving}
							onClick={() => void saveDraft()}
						>
							Save and connect
						</Button>
					{/if}
				{:else}
					<Button
						unifiedSize="sm"
						variant="accent"
						disabled={!canContinue}
						loading={connecting}
						endIcon={connecting ? undefined : { icon: ArrowRight }}
						onClick={next}
					>
						{step === 2 ? 'Connect' : 'Next'}
					</Button>
				{/if}
			</div>
		</div>
	</div>
</Modal2>

{#snippet providerCard(
	key: Provider,
	icon: Snippet,
	title: string,
	subtitle: string,
	eeOnly: boolean = false
)}
	{#snippet description()}
		<span class="flex items-center gap-2">
			<span>{subtitle}</span>
			{#if eeOnly}<EEOnly />{/if}
		</span>
	{/snippet}
	<RadioCard
		label={title}
		{description}
		selected={provider === key}
		disabled={eeOnly}
		{icon}
		showRadio={false}
		onSelect={() => selectProvider(key)}
	/>
{/snippet}
