<script lang="ts">
	import { Drawer, DrawerContent, Button } from '$lib/components/common'
	import SuperadminSettingsInner from './SuperadminSettingsInner.svelte'
	import Version from './Version.svelte'
	import MeltTooltip from './meltComponents/Tooltip.svelte'
	import Toggle from './Toggle.svelte'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import SaveButton from './SaveButton.svelte'
	import { X, FileDiff } from 'lucide-svelte'
	import { fade } from 'svelte/transition'
	import { SettingsService } from '$lib/gen'
	import { isCloudHosted } from '$lib/cloud'

	interface Props {
		disableChatOffset?: boolean
	}

	let { disableChatOffset = false }: Props = $props()

	let drawer: Drawer | undefined = $state()
	let innerComponent: SuperadminSettingsInner | undefined = $state()
	let uptodateVersion: string | undefined = $state(undefined)
	let yamlMode = $state(false)
	let diffMode = $state(false)
	let hasUnsavedChanges = $state(false)
	let pendingSave = $state(false)
	let showCloseConfirmModal = $state(false)

	async function loadUptodate() {
		try {
			const res = await SettingsService.backendUptodate()
			if (res != 'yes') {
				const parts = res.split(' -> ')
				uptodateVersion = parts.length > 1 ? parts[parts.length - 1] : res
			}
		} catch {}
	}
	loadUptodate()

	// When true, the next Drawer 'close' event is intentional and should not be intercepted
	let bypassCloseCheck = false

	export function openDrawer() {
		drawer?.openDrawer()
	}

	export function closeDrawer() {
		bypassCloseCheck = true
		drawer?.closeDrawer()
	}

	function handleClose() {
		if (hasUnsavedChanges) {
			showCloseConfirmModal = true
		} else {
			closeDrawer()
		}
	}

	/** Catches click-away and escape closes from the Drawer/Disposable layer */
	function handleDrawerClose() {
		if (!bypassCloseCheck && hasUnsavedChanges) {
			drawer?.openDrawer()
			showCloseConfirmModal = true
		}
		bypassCloseCheck = false
	}

	function handleSave(): void | Promise<void> {
		if (!pendingSave) {
			if (!innerComponent?.syncBeforeDiff()) return
			diffMode = true
			pendingSave = true
			return
		}
		return (async () => {
			await innerComponent?.saveSettings()
			diffMode = false
			pendingSave = false
		})()
	}

	function handleDiscard() {
		innerComponent?.discardAll()
		diffMode = false
		pendingSave = false
	}

	function handleShowDiff() {
		if (!diffMode) {
			if (!innerComponent?.syncBeforeDiff()) return
		}
		diffMode = !diffMode
		if (!diffMode) {
			pendingSave = false
		}
	}
</script>

<Drawer bind:this={drawer} size="1200px" {disableChatOffset} on:close={handleDrawerClose}>
	<DrawerContent noPadding overflow_y={false} title="Instance settings" on:close={handleClose}>
		{#snippet titleExtra()}
			<MeltTooltip disablePopup={!uptodateVersion}>
				<div class="text-xs text-secondary flex items-center gap-1 ml-6">
					<Version />
					{#if uptodateVersion}
						<span class="text-accent">→ {uptodateVersion}</span>
					{/if}
				</div>
				<svelte:fragment slot="text">
					{#if isCloudHosted()}
						The cloud version is updated daily.
					{:else}
						How to update?<br />
						- docker: <code>docker compose up -d</code><br />
						- <a href="https://github.com/windmill-labs/windmill-helm-charts#install">helm</a>
					{/if}
				</svelte:fragment>
			</MeltTooltip>
		{/snippet}
		{#snippet actions()}
			<div class="flex items-center gap-2">
				{#if hasUnsavedChanges}
					<div transition:fade={{ duration: 150 }}>
						<Button
							variant="default"
							size="xs"
							startIcon={{ icon: X }}
							onClick={handleDiscard}
						>
							Discard
						</Button>
					</div>
				{/if}
				{#if hasUnsavedChanges}
					<div transition:fade={{ duration: 150 }}>
						<Button
							variant={diffMode ? 'accent' : 'default'}
							size="xs"
							startIcon={{ icon: FileDiff }}
							onClick={handleShowDiff}
						>
							{diffMode ? 'Hide diff' : 'Show diff'}
						</Button>
					</div>
				{/if}
				<Toggle
					bind:checked={yamlMode}
					options={{ right: 'YAML' }}
					size="sm"
				/>
				<SaveButton
					onSave={handleSave}
					disabled={!hasUnsavedChanges}
					label={pendingSave ? 'Confirm & Save' : 'Save settings'}
					size="xs"
				/>
			</div>
		{/snippet}
		<SuperadminSettingsInner
			bind:this={innerComponent}
			closeDrawer={handleClose}
			showHeaderInfo={false}
			bind:yamlMode
			bind:diffMode
			bind:hasUnsavedChanges
		/>
	</DrawerContent>
</Drawer>

{#if showCloseConfirmModal}
	<ConfirmationModal
		open={showCloseConfirmModal}
		title="Unsaved changes"
		confirmationText="Discard & close"
		on:canceled={() => {
			showCloseConfirmModal = false
		}}
		on:confirmed={() => {
			innerComponent?.discardAll()
			showCloseConfirmModal = false
			diffMode = false
			pendingSave = false
			closeDrawer()
		}}
	>
		<div class="flex flex-col w-full space-y-4">
			<span>You have unsaved changes. Are you sure you want to discard them and close?</span>
			<Button
				variant="default"
				size="xs"
				startIcon={{ icon: FileDiff }}
				onClick={() => {
					showCloseConfirmModal = false
					diffMode = true
				}}
			>
				Show diff
			</Button>
		</div>
	</ConfirmationModal>
{/if}
