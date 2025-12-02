<script lang="ts">
	import { emptyString, type S3Object } from '$lib/utils'
	import { Button, Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import { tick, untrack } from 'svelte'
	import S3FilePickerInner from './S3FilePickerInner.svelte'
	import Select from './select/Select.svelte'
	import { FileUp } from 'lucide-svelte'
	import { usePromise } from '$lib/svelte5Utils.svelte'
	import { SettingService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'

	interface Props {
		fromWorkspaceSettings?: boolean
		readOnlyMode: boolean
		initialFileKey?: { s3: string; storage?: string } | undefined
		selectedFileKey?: { s3: string; storage?: string } | undefined
		folderOnly?: boolean
		regexFilter?: RegExp | undefined
		onClose?: () => void
		onSelectAndClose?: (selected: { s3: string; storage: string | undefined }) => void
	}

	let {
		fromWorkspaceSettings = false,
		readOnlyMode,
		initialFileKey = $bindable(undefined),
		selectedFileKey = $bindable(undefined),
		folderOnly = false,
		regexFilter = undefined,
		onClose,
		onSelectAndClose
	}: Props = $props()

	let drawer: Drawer | undefined = $state()
	let s3FilePickerInner: S3FilePickerInner | undefined = $state()

	let workspaceSettingsInitialized = $state(true)
	let storage: string | undefined = $state(undefined)
	let uploadModalOpen = $state(false)

	let allFilesByKey: Record<
		string,
		{
			type: 'folder' | 'leaf'
			full_key: string
			display_name: string
			collapsed: boolean
			parentPath: string | undefined
			nestingLevel: number
			count: number
		}
	> = $state({})

	let wasOpen = $state(false)

	let secondaryStorageNames = usePromise(
		() => SettingService.getSecondaryStorageNames({ workspace: $workspaceStore! }),
		{ loadInit: false }
	)

	$effect(() => {
		wasOpen && $workspaceStore && untrack(() => secondaryStorageNames.refresh())
	})

	export async function open(_preSelectedFileKey: S3Object | undefined = undefined) {
		drawer?.openDrawer?.()
		await tick()
		s3FilePickerInner?.open?.(_preSelectedFileKey)
	}
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<Drawer
	bind:this={drawer}
	on:close={() => {
		onClose?.()
		s3FilePickerInner?.close?.()
	}}
	size="1200px"
>
	<DrawerContent
		title="S3 file browser"
		on:close={() => {
			s3FilePickerInner?.exit?.()
			drawer?.closeDrawer?.()
		}}
		tooltip="Files present in the Workspace S3 bucket. You can set the workspace S3 bucket in the settings."
		documentationLink="https://www.windmill.dev/docs/integrations/s3"
	>
		<S3FilePickerInner
			bind:this={s3FilePickerInner}
			on:selectAndClose={(e) => {
				onSelectAndClose?.(e.detail)
				drawer?.closeDrawer?.()
			}}
			{fromWorkspaceSettings}
			{readOnlyMode}
			bind:initialFileKey
			bind:selectedFileKey
			bind:workspaceSettingsInitialized
			bind:storage
			bind:allFilesByKey
			bind:wasOpen
			bind:uploadModalOpen
			{folderOnly}
			{regexFilter}
		/>
		{#snippet actions()}
			<div class="flex gap-1">
				{#if secondaryStorageNames.value?.length}
					<Select
						inputClass="h-10 min-w-44 !placeholder-secondary"
						items={[
							{ value: undefined, label: 'Default storage' },
							...secondaryStorageNames.value.map((value) => ({ value }))
						]}
						placeholder="Default storage"
						bind:value={
							() => storage,
							(v) => {
								if (v === storage) return
								storage = v
								s3FilePickerInner?.reloadContent()
							}
						}
					/>
				{/if}
				{#if !readOnlyMode}
					<Button
						variant="border"
						color="light"
						disabled={workspaceSettingsInitialized === false}
						startIcon={{ icon: FileUp }}
						on:click={() => {
							uploadModalOpen = true
						}}>Upload File</Button
					>
					{#if !fromWorkspaceSettings && s3FilePickerInner}
						<Button
							disabled={selectedFileKey === undefined ||
								emptyString(selectedFileKey.s3) ||
								(folderOnly && allFilesByKey[selectedFileKey.s3]?.type !== 'folder')}
							on:click={s3FilePickerInner.selectAndClose}>Select</Button
						>
					{/if}
				{/if}
			</div>
		{/snippet}
	</DrawerContent>
</Drawer>
