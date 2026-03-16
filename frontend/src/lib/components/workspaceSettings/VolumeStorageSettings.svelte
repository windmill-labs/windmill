<script lang="ts">
	import { goto } from '$lib/navigation'
	import { page } from '$app/stores'
	import { workspaceStore } from '$lib/stores'
	import { emptyString, sendUserToast } from '$lib/utils'
	import SettingsPageHeader from '../settings/SettingsPageHeader.svelte'
	import SettingsFooter from './SettingsFooter.svelte'
	import Select from '../select/Select.svelte'
	import { Button } from '../common'
	import Alert from '../common/alert/Alert.svelte'
	import { convertFrontendToBackendSetting, type S3ResourceSettings } from '$lib/workspace_settings'
	import { WorkspaceService } from '$lib/gen'

	let {
		s3ResourceSettings = $bindable(),
		s3ResourceSavedSettings,
		onSave = undefined,
		onDiscard = undefined
	}: {
		s3ResourceSettings: S3ResourceSettings
		s3ResourceSavedSettings: S3ResourceSettings
		onSave?: () => void
		onDiscard?: () => void
	} = $props()

	async function saveVolumeStorageSettings(): Promise<void> {
		const large_file_storage = convertFrontendToBackendSetting(s3ResourceSettings)
		await WorkspaceService.editLargeFileStorageConfig({
			workspace: $workspaceStore!,
			requestBody: {
				large_file_storage: large_file_storage
			}
		})
		sendUserToast('Volume storage settings saved')
		onSave?.()
	}

	let hasUnsavedChanges = $derived(
		s3ResourceSettings.volumeStorage !== s3ResourceSavedSettings.volumeStorage
	)

	let volumeStorageItems: { value: string; label: string }[] = $derived.by(() => {
		const items: { value: string; label: string }[] = [{ value: '', label: 'Disabled' }]
		if (!emptyString(s3ResourceSettings.resourcePath)) {
			items.push({ value: 'primary', label: 'Primary storage' })
		}
		for (const [name, s] of s3ResourceSettings.secondaryStorage ?? []) {
			if (!emptyString(s.resourcePath)) {
				items.push({ value: name, label: name })
			}
		}
		return items
	})

	let hasAvailableStorage = $derived(volumeStorageItems.length > 1)

	function goToStorageSettings() {
		const params = new URLSearchParams($page.url.searchParams)
		params.set('tab', 'windmill_lfs')
		goto(`?${params.toString()}`)
	}
</script>

<SettingsPageHeader
	title="Volume storage"
	description="Select which storage volumes should use. If disabled, scripts with volumes will fail with an error."
	link="https://www.windmill.dev/docs/core_concepts/persistent_storage/volumes"
/>
{#if s3ResourceSettings}
	{#if hasAvailableStorage}
		<div class="max-w-sm mt-2">
			<Select
				items={volumeStorageItems}
				bind:value={
					() => s3ResourceSettings.volumeStorage ?? '',
					(v) => {
						s3ResourceSettings.volumeStorage = v || undefined
					}
				}
			/>
		</div>

		<SettingsFooter
			class="mt-5 mb-5"
			inline
			{hasUnsavedChanges}
			onSave={saveVolumeStorageSettings}
			onDiscard={() => onDiscard?.()}
			saveLabel="Save volume storage settings"
		/>
	{:else}
		<Alert type="info" title="No workspace storage configured" class="mt-4">
			You need to configure a workspace object storage before you can use volumes.
		</Alert>
		<Button wrapperClasses="mt-2" variant="default" size="sm" on:click={goToStorageSettings}>
			Go to Object storage settings
		</Button>
	{/if}
{/if}
