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
	import GithubIcon from '../icons/GithubIcon.svelte'
	import GitlabIcon from '../icons/GitlabIcon.svelte'
	import GitlabProjectConnectForm from './setup/GitlabProjectConnectForm.svelte'
	import GithubAppConnectForm from './setup/GithubAppConnectForm.svelte'
	import TokenUrlConnectForm from './setup/TokenUrlConnectForm.svelte'
	import { createRepositoryResource, repoSlug, type RepoConnection } from './setup/repoConnection'
	import { GithubAppSetup } from './setup/githubAppSetup.svelte'
	import { onDestroy } from 'svelte'
	import { ResourceService } from '$lib/gen'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { apiErrorMessage } from '$lib/utils'

	type Provider = 'gitlab' | 'github_app' | 'github_pat' | 'existing'

	type Props = {
		opened: boolean
		title: string
		/** Resources already attached to a repository of this workspace, which cannot be picked twice. */
		usedResourcePaths: string[]
		/** The git_repository resource the repository is to be synced with, picked or just created. */
		onResource: (path: string) => void
	}

	let { opened = $bindable(), title, usedResourcePaths, onResource }: Props = $props()

	const STEPS = ['Choose a provider', 'Connect the repository', 'Configure sync']

	let step: 1 | 2 = $state(1)
	let provider: Provider | undefined = $state(undefined)
	let existingPath: string | undefined = $state(undefined)

	let connection: RepoConnection | undefined = $state(undefined)
	let newPath = $state('')
	let newPathError = $state('')
	let branch = $state('')
	let folder = $state('')
	let connecting = $state(false)
	let connectError: string | undefined = $state(undefined)
	let githubApp: GithubAppSetup | undefined = $state(undefined)

	onDestroy(() => githubApp?.dispose())
	$effect(() => {
		if (!opened) untrack(() => githubApp?.dispose())
	})

	async function reset() {
		step = 1
		githubApp?.dispose()
		githubApp = undefined
		provider = undefined
		existingPath = undefined
		connection = undefined
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

	// GitLab without EE has no server-side token store nor project listing, so it
	// takes the same URL + token form as a GitHub PAT.
	const gitlabHeld = $derived(provider === 'gitlab' && !!$enterpriseLicense)

	const canContinue = $derived(
		step === 1
			? provider === 'existing'
				? !!existingPath
				: provider !== undefined
			: !!connection && !!newPath && !newPathError && !connecting
	)

	function selectProvider(p: Provider) {
		if (p !== provider) connection = undefined
		provider = p
		if (p === 'github_app' && !githubApp && $workspaceStore) {
			githubApp = new GithubAppSetup($workspaceStore)
			void githubApp.reload()
		}
	}

	function finish(path: string) {
		opened = false
		onResource(path)
	}

	async function next() {
		if (step === 1) {
			if (provider === 'existing') {
				if (existingPath) finish(existingPath)
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
		try {
			await createRepositoryResource(workspace, newPath, connection, { branch, folder })
			finish(newPath)
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
			small
			on:click={(e) => {
				if (e.detail.index === 0 && !connecting) step = 1
			}}
		/>

		<div class="flex-1 flex flex-col min-h-0">
			<div class="flex-1 overflow-y-auto flex flex-col gap-2">
				{#if step === 1}
					{#snippet githubIcon()}
						<GithubIcon height={18} width={18} />
					{/snippet}
					{@render providerCard(
						'github_app',
						githubIcon,
						'GitHub App',
						'Install the Windmill GitHub App on your repository. Enables webhooks, pull requests and commit checks.',
						!$enterpriseLicense
					)}
					{@render providerCard(
						'github_pat',
						githubIcon,
						'GitHub personal access token',
						'Connect a GitHub repository with a fine-grained personal access token.'
					)}
					{#snippet gitlabIcon()}
						<GitlabIcon height={18} width={18} />
					{/snippet}
					{@render providerCard(
						'gitlab',
						gitlabIcon,
						'GitLab',
						'Connect a GitLab repository with an access token.'
					)}

					{#if provider === 'existing'}
						{#snippet resourceIcon()}
							<Database size={18} class="text-secondary" />
						{/snippet}
						{@render providerCard(
							'existing',
							resourceIcon,
							'Use an existing git_repository resource',
							'Pick a resource that already holds the repository URL and its credentials.'
						)}
						<ResourcePicker
							bind:value={existingPath}
							resourceType="git_repository"
							excludedValues={usedResourcePaths}
						/>
					{:else}
						<div>
							<Button variant="subtle" unifiedSize="xs" onClick={() => selectProvider('existing')}>
								Use an existing resource instead
							</Button>
						</div>
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
					{#if step === 2}
						<Button
							unifiedSize="sm"
							variant="default"
							disabled={connecting}
							onClick={() => (step = 1)}
						>
							Back
						</Button>
					{/if}
				</div>
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
	{@const selected = provider === key}
	<button
		class="text-left border rounded-md p-3 flex gap-3 items-start transition-colors {selected
			? 'border-border-selected/50 bg-surface-accent-selected'
			: 'border-border-light hover:bg-surface-hover'} disabled:opacity-60 disabled:cursor-not-allowed"
		disabled={eeOnly}
		onclick={() => selectProvider(key)}
	>
		<span class="mt-0.5 shrink-0">{@render icon()}</span>
		<span class="flex flex-col gap-0.5 min-w-0">
			<span
				class="text-xs font-medium flex items-center gap-2 {selected
					? 'text-accent'
					: 'text-emphasis'}"
			>
				{title}
				{#if eeOnly}<EEOnly />{/if}
			</span>
			<span class="text-xs text-secondary font-normal">{subtitle}</span>
		</span>
	</button>
{/snippet}
