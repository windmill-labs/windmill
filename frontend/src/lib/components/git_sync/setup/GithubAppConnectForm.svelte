<script lang="ts">
	import { Download, ExternalLink, Loader2, Minus, RotateCw, Upload } from 'lucide-svelte'
	import { Alert, Button } from '$lib/components/common'
	import Select from '$lib/components/select/Select.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import RepositorySelector from '$lib/components/RepositorySelector.svelte'
	import {
		addInstallationToWorkspace,
		deleteInstallation,
		exportInstallation,
		importInstallation
	} from '$lib/githubApp'
	import type { RepoConnection } from './repoConnection'
	import type { GithubAppSetup } from './githubAppSetup.svelte'

	type Props = {
		workspace: string
		setup: GithubAppSetup
		connection: RepoConnection | undefined
	}

	let { workspace, setup, connection = $bindable() }: Props = $props()

	const gh = $derived(setup.gh)
	const usable = $derived(setup.usable)
	const broken = $derived(gh.workspaceGithubInstallations.filter((i) => !!i.error))
	// Installations reachable from another workspace the user is in, deduplicated.
	const elsewhere = $derived(
		gh.githubInstallations.filter(
			(i, idx, all) =>
				!i.error &&
				!gh.workspaceGithubInstallations.some((w) => w.installation_id === i.installation_id) &&
				all.findIndex((o) => o.installation_id === i.installation_id) === idx
		)
	)
	const selected = $derived(
		usable.find((i) => i.installation_id === gh.selectedGHAppInstallationId)
	)

	const reload = () => setup.reload()

	$effect(() => {
		connection = gh.selectedGHAppRepository
			? { url: gh.selectedGHAppRepository, isGithubApp: true }
			: undefined
	})

	async function useFrom(installationId: number, sourceWorkspace: string) {
		await addInstallationToWorkspace(workspace, installationId, sourceWorkspace, reload)
	}

	/** Pasting a JWT exported from another Windmill instance. */
	let importing = $state(false)

	// Export hands out a JWT the other instance imports; an installation reached through a
	// GitHub Enterprise Server of its own is not transferable that way.
	const installationActions = $derived([
		...(selected && !selected.github_base_url
			? [
					{
						displayName: 'Copy installation token (to another instance)',
						icon: Download,
						action: () => void exportInstallation(workspace, selected.installation_id)
					}
				]
			: []),
		...(gh.isGhesSelfManaged
			? []
			: [
					{
						displayName: 'Import an installation from another instance',
						icon: Upload,
						action: () => (importing = true)
					}
				]),
		...(selected && !selected.provisioned_by_admin
			? [
					{
						displayName: 'Remove installation from this workspace',
						icon: Minus,
						type: 'delete' as const,
						action: () => void deleteInstallation(workspace, selected.installation_id, reload)
					}
				]
			: [])
	])
</script>

<div class="flex flex-col gap-4">
	{#if (!setup.loaded || gh.loadingGithubInstallations) && gh.workspaceGithubInstallations.length === 0}
		<div class="flex items-center gap-2 text-xs text-secondary">
			<Loader2 size={14} class="animate-spin" /> Loading GitHub App installations...
		</div>
	{:else}
		{#if usable.length === 0}
			<Alert type="info" size="xs" bgClass="border-0" title="">
				{#if gh.isCheckingInstallation}
					Finish installing the Windmill GitHub App in the tab that opened. This dialog picks up the
					installation on its own.
				{:else}
					Install the Windmill GitHub App on the GitHub account or organization that owns the
					repository, then come back here.
				{/if}
			</Alert>
		{:else}
			<label class="flex flex-col gap-1">
				<span class="text-xs font-semibold text-emphasis">GitHub account</span>
				<Select
					items={usable.map((i) => ({
						label: i.account_id,
						value: i.installation_id,
						subtitle: `${i.total_count} repositories`
					}))}
					bind:value={
						() => gh.selectedGHAppInstallationId,
						(v) => {
							gh.selectedGHAppInstallationId = v
							gh.selectedGHAppRepository = undefined
						}
					}
					clearable={false}
				/>
			</label>
			{#if selected}
				<label class="flex flex-col gap-1">
					<span class="text-xs font-semibold text-emphasis">Repository</span>
					<!-- RepositorySelector snapshots its repositories at mount -->
					{#key selected.installation_id}
						<RepositorySelector
							bind:selectedRepository={gh.selectedGHAppRepository}
							installationId={selected.installation_id}
							initialRepositories={selected.repositories}
							totalCount={selected.total_count}
							perPage={selected.per_page}
						/>
					{/key}
				</label>
			{/if}
		{/if}

		{#if broken.length > 0}
			<Alert type="warning" size="xs" title="Some installations cannot be used">
				{broken.map((i) => `${i.account_id}: ${i.error}`).join('; ')}
			</Alert>
		{/if}

		<div class="flex items-center gap-2">
			<Button
				variant="default"
				unifiedSize="sm"
				disabled={!gh.githubInstallationUrl}
				startIcon={gh.isCheckingInstallation
					? { icon: Loader2, classes: 'animate-spin' }
					: undefined}
				endIcon={{ icon: ExternalLink }}
				title={gh.isCheckingInstallation ? 'Open the GitHub installation page again' : undefined}
				onClick={() => setup.openInstall()}
			>
				{gh.isCheckingInstallation
					? 'Waiting for the installation...'
					: usable.length === 0
						? 'Install the GitHub App'
						: 'Install on another account'}
			</Button>
			<Button
				variant="subtle"
				unifiedSize="sm"
				iconOnly
				title="Refresh installations"
				startIcon={{ icon: RotateCw }}
				disabled={gh.loadingGithubInstallations}
				onClick={reload}
			/>
			{#if installationActions.length > 0}
				<DropdownV2 items={installationActions} />
			{/if}
		</div>

		{#if importing}
			<div class="flex flex-col gap-1">
				<span class="text-xs font-semibold text-emphasis">Installation token</span>
				<span class="text-xs text-secondary">
					Paste the token copied from the instance that holds the installation.
				</span>
				<div class="flex items-center gap-2">
					<TextInput
						bind:value={gh.importJwt}
						size="sm"
						class="flex-1 min-w-0"
						inputProps={{ placeholder: 'JWT token', autocomplete: 'off' }}
					/>
					<Button
						variant="default"
						unifiedSize="sm"
						disabled={!gh.importJwt}
						onClick={async () => {
							await importInstallation(workspace, gh.importJwt, () => {
								gh.importJwt = ''
								importing = false
								void reload()
							})
						}}
					>
						Import
					</Button>
				</div>
			</div>
		{/if}

		{#if elsewhere.length > 0}
			<div class="flex flex-col gap-1">
				<span class="text-xs font-semibold text-emphasis">Installed in your other workspaces</span>
				{#each elsewhere as inst (inst.installation_id)}
					<div class="flex items-center justify-between gap-2 text-xs">
						<span class="text-primary">
							{inst.account_id}
							<span class="text-secondary">from {inst.workspace_id}</span>
						</span>
						<Button
							variant="default"
							unifiedSize="xs"
							onClick={() => inst.workspace_id && useFrom(inst.installation_id, inst.workspace_id)}
						>
							Use here
						</Button>
					</div>
				{/each}
			</div>
		{/if}
	{/if}
</div>
