<script lang="ts">
	import { goto } from '$lib/navigation'
	import { page } from '$app/stores'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import InstanceSettings from '$lib/components/InstanceSettings.svelte'
	import { Button } from '$lib/components/common'
	import SidebarNavigation from '$lib/components/common/sidebar/SidebarNavigation.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import {
		setupNavigationGroups,
		tabToCategoryMap,
		tabToAuthSubTab,
		categoryToTabMap
	} from '$lib/components/instanceSettings'
	import Breadcrumb from '$lib/components/common/breadcrumb/Breadcrumb.svelte'
	import { ChevronRight, ArrowLeft } from 'lucide-svelte'

	const wizardSteps = [
		{ id: 'Core', label: 'General' },
		{ id: 'Auth/OAuth/SAML', label: 'Authentication' }
	] as const

	const initialMode = $page.url.searchParams.get('mode') === 'full' ? 'full' : 'wizard'
	let mode: 'wizard' | 'full' = $state(initialMode)
	let wizardStep = $state(0)

	let instanceSettings: InstanceSettings | undefined = $state()
	let currentStepDirty = $derived(instanceSettings?.isDirty(wizardSteps[wizardStep].id) ?? false)

	// --- Full settings mode state ---
	let fullTab = $state('general')
	let instanceSettingsCategory = $derived(tabToCategoryMap[fullTab] ?? 'Core')
	let authSubTab: 'sso' | 'oauth' | 'scim' = $derived(tabToAuthSubTab[fullTab] ?? 'sso')

	// --- Unsaved changes detection (full mode) ---
	let pendingTab: string | undefined = $state(undefined)
	let showUnsavedChangesModal = $state(false)

	function handleNavigate(newTab: string) {
		if (newTab === fullTab) return
		const currentCategory = tabToCategoryMap[fullTab]
		if (currentCategory && instanceSettings?.isDirty(currentCategory)) {
			pendingTab = newTab
			showUnsavedChangesModal = true
		} else {
			fullTab = newTab
		}
	}

	/** Auto-save the current wizard step if dirty, then run the callback */
	async function saveAndProceed(callback: () => void) {
		const category = wizardSteps[wizardStep].id
		if (instanceSettings?.isDirty(category)) {
			await instanceSettings.saveCategorySettings(category)
		}
		callback()
	}

	function switchToFullMode() {
		mode = 'full'
	}

	function switchToWizardMode() {
		mode = 'wizard'
	}

	function finishSetup() {
		goto('/apps/get/g/all/setup_app?nomenubar=true&workspace=admins')
	}
</script>

<CenteredModal large title="Instance settings" centerVertically={false} containOverflow>
	<div class="flex flex-col flex-1 min-h-0 overflow-hidden">
		{#if mode === 'wizard'}
			<!-- Step indicator (pinned top) -->
			<div class="pb-2 border-b shrink-0 flex justify-start">
				<Breadcrumb
					items={wizardSteps.map((s) => s.label)}
					selectedIndex={wizardStep + 1}
					numbered
					onselect={(i) => {
						if (i < wizardStep) saveAndProceed(() => (wizardStep = i))
					}}
				>
					{#snippet separator()}
						<ChevronRight size={16} class="text-tertiary shrink-0" />
					{/snippet}
				</Breadcrumb>
			</div>

			<!-- Step content (scrollable) -->
			<div class="flex-1 overflow-auto min-h-0 pt-4">
				{#if wizardSteps[wizardStep].id === 'Auth/OAuth/SAML'}
					<p class="text-secondary text-xs mb-4">
						Windmill uses its own authentication by default. SSO configuration is optional and can
						be set up later.
					</p>
				{/if}
				{#key wizardStep}
					<InstanceSettings
						bind:this={instanceSettings}
						hideTabs
						quickSetup
						tab={wizardSteps[wizardStep].id}
					/>
				{/key}
			</div>
		{:else}
			<!-- Sidebar + Content -->
			<div class="flex flex-1 min-h-0">
				<div class="w-44 shrink-0 overflow-auto pb-4 pr-4">
					<SidebarNavigation
						groups={setupNavigationGroups}
						selectedId={fullTab}
						onNavigate={handleNavigate}
					/>
				</div>

				<div class="flex-1 min-w-0 overflow-auto px-4">
					<InstanceSettings
						bind:this={instanceSettings}
						hideTabs
						tab={instanceSettingsCategory}
						{authSubTab}
						onNavigateToTab={(category) => {
							const targetTab = categoryToTabMap[category]
							if (targetTab) {
								handleNavigate(targetTab)
							}
						}}
					/>
				</div>
			</div>
		{/if}

		<!-- Navigation (pinned bottom) -->
		<div class="flex items-center justify-between pt-4 border-t shrink-0">
			{#if mode === 'wizard'}
				<div class="flex items-center gap-2">
					{#if wizardStep > 0}
						<Button
							variant="default"
							unifiedSize="md"
							startIcon={{ icon: ArrowLeft }}
							on:click={() => saveAndProceed(() => (wizardStep -= 1))}
						>
							Back
						</Button>
					{/if}
				</div>

				<div class="flex items-center gap-2">
					<Button
						variant="default"
						unifiedSize="md"
						onClick={() => saveAndProceed(switchToFullMode)}
					>
						Advanced setup
					</Button>
					{#if wizardStep < wizardSteps.length - 1}
						<Button
							variant="accent"
							unifiedSize="md"
							on:click={() => saveAndProceed(() => (wizardStep += 1))}
						>
							{currentStepDirty ? 'Save & Next' : 'Next'}
						</Button>
					{:else}
						<Button variant="accent" unifiedSize="md" on:click={() => saveAndProceed(finishSetup)}>
							{currentStepDirty ? 'Save & Continue' : 'Continue'}
						</Button>
					{/if}
				</div>
			{:else}
				<Button
					variant="default"
					unifiedSize="md"
					startIcon={{ icon: ArrowLeft }}
					on:click={switchToWizardMode}
				>
					Quick setup
				</Button>
				<Button variant="accent" unifiedSize="md" on:click={finishSetup}>Continue</Button>
			{/if}
		</div>

		<div class="flex items-center justify-start gap-2 mt-2 shrink-0">
			<p class="text-secondary text-xs">
				You can change these settings later in the instance settings.
			</p>
			<Button variant="subtle" unifiedSize="sm" on:click={finishSetup}>Skip setup</Button>
		</div>
	</div>
</CenteredModal>

{#if showUnsavedChangesModal}
	<ConfirmationModal
		open={showUnsavedChangesModal}
		title="Unsaved changes detected"
		confirmationText="Discard changes"
		on:canceled={() => {
			showUnsavedChangesModal = false
			pendingTab = undefined
		}}
		on:confirmed={() => {
			if (pendingTab !== undefined) {
				const currentCategory = tabToCategoryMap[fullTab]
				if (currentCategory) {
					instanceSettings?.discardCategory(currentCategory)
				}
				fullTab = pendingTab
			}
			showUnsavedChangesModal = false
			pendingTab = undefined
		}}
	>
		<div class="flex flex-col w-full space-y-4">
			<span>You have unsaved changes. Are you sure you want to discard them?</span>
		</div>
	</ConfirmationModal>
{/if}
