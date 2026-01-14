<script lang="ts">
	import { Section, Alert } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import Description from '$lib/components/Description.svelte'
	import { enterpriseLicense } from '$lib/stores'

	// Protection rules state
	let requireForkOrBranch = $state(false)
	let disableFork = $state(false)
	let disableMergeUI = $state(false)
	let disableExecution = $state(false)
	let adminsBypassDisabled = $state(false)

	// Track initial state for unsaved changes detection
	let initialRequireForkOrBranch = $state(false)
	let initialDisableFork = $state(false)
	let initialDisableMergeUI = $state(false)
	let initialDisableExecution = $state(false)
	let initialAdminsBypassDisabled = $state(false)

	// TODO: Load settings from backend
	// TODO: Implement save functionality

	const hasUnsavedChanges = $derived(
		requireForkOrBranch !== initialRequireForkOrBranch ||
			disableFork !== initialDisableFork ||
			disableMergeUI !== initialDisableMergeUI ||
			disableExecution !== initialDisableExecution ||
			adminsBypassDisabled !== initialAdminsBypassDisabled
	)
</script>

<div class="flex flex-col gap-4 my-8">
	<div class="flex flex-col gap-1">
		<div class="text-sm font-semibold text-emphasis">Workspace Protection Rules</div>
		<Description>
			Configure protection rules to control how users can interact with this workspace. These
			rules help enforce governance and security policies.
		</Description>
	</div>
</div>

{#if !$enterpriseLicense}
	<Alert type="warning" title="Workspace Protection Rules is an EE feature">
		Workspace Protection Rules is a Windmill Enterprise Edition feature. It enables enforcing
		governance and security policies across your workspace.
	</Alert>
	<div class="pb-4"></div>
{/if}

<Section
	label="Access Control Rules"
	description="Configure rules that control how users can modify and execute content in this workspace."
	class="space-y-6"
>
	<div class="flex flex-col gap-6">
		<!-- Require Fork or Branch -->
		<div class="flex flex-col gap-2">
			<Toggle
				disabled={!$enterpriseLicense}
				bind:checked={requireForkOrBranch}
				options={{
					right: 'Require fork or git branch for changes'
				}}
			/>
			<div class="text-xs text-secondary ml-6">
				When enabled, users must use a fork or git branch to make changes. Direct edits to the
				main workspace content are not allowed.
			</div>
		</div>

		<!-- Disable Fork -->
		<div class="flex flex-col gap-2">
			<Toggle
				disabled={!$enterpriseLicense}
				bind:checked={disableFork}
				options={{
					right: 'Disable workspace forking'
				}}
			/>
			<div class="text-xs text-secondary ml-6">
				When enabled, users cannot create forks of this workspace.
			</div>
		</div>

		<!-- Disable Merge UI -->
		<div class="flex flex-col gap-2">
			<Toggle
				disabled={!$enterpriseLicense}
				bind:checked={disableMergeUI}
				options={{
					right: 'Disable merge UI for forks'
				}}
			/>
			<div class="text-xs text-secondary ml-6">
				When enabled, users cannot deploy fork changes through the web UI. Merges must be done through
				external processes such as a PR on the Git Sync repo.
			</div>
		</div>

		<!-- Disable Execution -->
		<div class="flex flex-col gap-2">
			<Toggle
				disabled={!$enterpriseLicense}
				bind:checked={disableExecution}
				options={{
					right: 'Disable manual script and flow execution'
				}}
			/>
			<div class="text-xs text-secondary ml-6">
				When enabled, users cannot execute scripts or flows in this workspace. This is useful for
				read-only or review workspaces.
			</div>
		</div>

		<!-- Admin Bypass -->
		<div class="flex flex-col gap-2">
			<Toggle
				disabled={!$enterpriseLicense}
				bind:checked={adminsBypassDisabled}
				options={{
					right: 'Disable admin bypass for these rules'
				}}
			/>
			<div class="text-xs text-secondary ml-6">
				When enabled, even workspace admins must follow these protection rules. Use this for
				strict governance requirements.
			</div>
		</div>
	</div>
</Section>

{#if hasUnsavedChanges && $enterpriseLicense}
	<div class="mt-6">
		<Alert type="info" title="Unsaved changes">
			You have unsaved changes to your protection rules. Remember to save your changes before
			leaving this page.
		</Alert>
	</div>
{/if}
