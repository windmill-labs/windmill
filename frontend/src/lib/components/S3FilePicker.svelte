<script lang="ts">
	import { type S3Object } from '$lib/utils'
	import { Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import { createEventDispatcher, tick } from 'svelte'
	import S3FilePickerInner from './S3FilePickerInner.svelte'

	let dispatch = createEventDispatcher<{
		close: { s3: string; storage: string | undefined } | undefined
		selectAndClose: { s3: string; storage: string | undefined }
	}>()

	interface Props {
		fromWorkspaceSettings?: boolean
		readOnlyMode: boolean
		initialFileKey?: { s3: string; storage?: string } | undefined
		selectedFileKey?: { s3: string; storage?: string } | undefined
		folderOnly?: boolean
		regexFilter?: RegExp | undefined
	}

	let {
		fromWorkspaceSettings = false,
		readOnlyMode,
		initialFileKey = $bindable(undefined),
		selectedFileKey = $bindable(undefined),
		folderOnly = false,
		regexFilter = undefined
	}: Props = $props()

	let drawer: Drawer | undefined = $state()
	let s3FilePickerInner: S3FilePickerInner | undefined = $state()

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
		dispatch('close')
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
				dispatch('selectAndClose', e.detail)
				drawer?.closeDrawer?.()
			}}
			{fromWorkspaceSettings}
			{readOnlyMode}
			bind:initialFileKey
			bind:selectedFileKey
			{folderOnly}
			{regexFilter}
		/>
	</DrawerContent>
</Drawer>
