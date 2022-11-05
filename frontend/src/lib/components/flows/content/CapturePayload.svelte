<script lang="ts">
	import { page } from '$app/stores'
	import { Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import WindmillIcon from '$lib/components/icons/WindmillIcon.svelte'
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'

	import { CaptureService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { flowStore } from '../flowStore'

	import { convert } from '@redocly/json-to-json-schema'
	import SchemaViewer from '$lib/components/SchemaViewer.svelte'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'

	const { previewArgs } = getContext<FlowEditorContext>('FlowEditorContext')

	let drawer: Drawer
	let interval: NodeJS.Timeout | undefined = undefined
	let captureInput: any | undefined = undefined
	let jsonSchema: any = undefined

	export function openDrawer() {
		drawer.openDrawer()
	}

	async function startCapturePoint() {
		await CaptureService.createCapture({
			workspace: $workspaceStore!,
			path: $flowStore.path
		})
	}

	async function getCaptureInput() {
		const capture = await CaptureService.getCapture({
			workspace: $workspaceStore!,
			path: $flowStore.path
		})
		captureInput = capture
		jsonSchema = { required: [], ...convert(capture) }
	}
</script>

<Drawer
	bind:this={drawer}
	on:open={() => {
		startCapturePoint()
		interval = setInterval(() => {
			getCaptureInput()
		}, 1000)
	}}
	on:close={() => interval && clearInterval(interval)}
>
	<DrawerContent title="Capture from a request to seed inputs" on:close={drawer.closeDrawer}>
		Send a payload at: <div
			><a
				class="text-2xl"
				href="{$page.url.protocol}//{$page.url
					.hostname}/api/w/{$workspaceStore}/capture/{$flowStore.path}"
				>{$page.url.protocol}//{$page.url
					.hostname}/api/w/{$workspaceStore}/capture/{$flowStore.path}</a
			></div
		>
		<div class="items-center flex flex-row gap-x-2 text-xs text-gray-600">
			Listening for new payload
			<WindmillIcon
				class="animate-[pulse_5s_linear_infinite] animate-[spin_5s_linear_infinite]"
			/></div
		>
		<div class="box p-2 my-2">
			<ObjectViewer topBrackets={true} json={captureInput} />
		</div>
		<div class="flex flex-row gap-2">
			<Button
				size="sm"
				on:click={() => {
					$previewArgs = captureInput
					$flowStore.schema = jsonSchema
				}}>Copy as flow inputs and test args</Button
			>
			<Button
				size="sm"
				variant="border"
				on:click={() => {
					$previewArgs = captureInput
				}}>Copy only as test args</Button
			>
		</div>
		<h3 class="mt-2">JSONSchema</h3>
		<div class="box p-2">
			<SchemaViewer schema={jsonSchema} />
		</div>
	</DrawerContent>
</Drawer>
