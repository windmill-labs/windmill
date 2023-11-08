<script lang="ts">
	import { BROWSER } from 'esm-env'
	import { Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'

	import { CaptureService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'

	import { convert } from '@redocly/json-to-json-schema'
	import SchemaViewer from '$lib/components/SchemaViewer.svelte'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import { copyToClipboard, sendUserToast } from '$lib/utils'

	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import { Clipboard } from 'lucide-svelte'

	const { previewArgs, flowStore, pathStore } = getContext<FlowEditorContext>('FlowEditorContext')

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
			path: $pathStore
		})
	}

	async function getCaptureInput() {
		const capture = await CaptureService.getCapture({
			workspace: $workspaceStore!,
			path: $pathStore
		})
		captureInput = capture
		jsonSchema = { required: [], properties: {}, ...convert(capture) }
	}

	let hostname = BROWSER ? window.location.protocol + '//' + window.location.host : 'SSR'
</script>

<Drawer
	bind:this={drawer}
	size="900px"
	on:open={() => {
		startCapturePoint()
		interval = setInterval(() => {
			getCaptureInput()
		}, 1000)
	}}
	on:close={() => interval && clearInterval(interval)}
>
	<DrawerContent title="Capture request" on:close={drawer.closeDrawer}>
		Send a payload at: <div>
			<a
				class="text-xs border p-2 flex flex-row items-center gap-2 justify-between"
				on:click={(e) => {
					e.preventDefault()
					copyToClipboard(`${hostname}/api/w/${$workspaceStore}/capture_u/${$pathStore}`)
				}}
				href="{hostname}/api/w/{$workspaceStore}/capture_u/{$pathStore}"
			>
				{hostname}/api/w/{$workspaceStore}/capture_u/{$pathStore}
				<Clipboard size={14} />
			</a>
		</div>
		<p class="text-secondary mt-4 text-xs">CURL example</p>

		<div class="text-xs box mb-4 b">
			<pre class="overflow-auto"
				>{`curl -X POST ${hostname}/api/w/${$workspaceStore}/capture_u/${$pathStore} \\
   -H 'Content-Type: application/json' \\
   -d '{"foo": 42}'`}</pre
			>
		</div>
		<div class="items-center flex flex-row gap-x-2 text-xs text-secondary">
			Listening for new requests
			<span class="relative flex h-3 w-3">
				<span
					class="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75"
				/>
				<span class="relative inline-flex rounded-full h-3 w-3 bg-green-500" />
			</span>
		</div>
		<div class="box p-2 my-2 mb-4">
			<ObjectViewer topBrackets={true} json={captureInput} />
		</div>
		<svelte:fragment slot="actions">
			<Button
				size="sm"
				variant="border"
				on:click={() => {
					$previewArgs = captureInput
					sendUserToast('Copied as test args')
				}}
			>
				Copy only as test args
			</Button>
			<Button
				size="sm"
				on:click={() => {
					$previewArgs = captureInput
					$flowStore.schema = jsonSchema
					sendUserToast('Copied as flow inputs and test args')
				}}
			>
				Copy as flow inputs and test args
			</Button>
		</svelte:fragment>
		<h3 class="my-2 mt-8">Derived schema</h3>
		<div class="box p-2">
			<SchemaViewer schema={jsonSchema} />
		</div>
		<h3 class="mt-8">Test args</h3>
		<SchemaForm class="pt-4" schema={$flowStore.schema} args={$previewArgs} />
	</DrawerContent>
</Drawer>
