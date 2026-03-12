<script lang="ts">
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import { filteredContentForExport } from '../utils.svelte'
	import YAML from 'yaml'

	import { Button } from '$lib/components/common'
	import { sendUserToast } from '$lib/toast'
	import { Loader2 } from 'lucide-svelte'
	import { refreshStateStore } from '$lib/svelte5Utils.svelte'
	import SimpleEditor from '../../SimpleEditor.svelte'

	const { flowStore } = $state(getContext<FlowEditorContext>('FlowEditorContext'))

	interface Props {
		drawer: Drawer | undefined
	}

	let { drawer = $bindable() }: Props = $props()

	let code = $state('')
	let initialCode = $state('')
	let editor = $state(undefined) as SimpleEditor | undefined
	let hasChanges = $derived(code !== initialCode)

	function reload() {
		code = YAML.stringify(filteredContentForExport(flowStore.val))
		initialCode = code
		editor?.setCode(code)
	}

	function apply() {
		try {
			const parsed = YAML.parse(code)
			if (parsed.summary && typeof parsed.summary === 'string') {
				flowStore.val.summary = parsed.summary
			}
			if (parsed.description && typeof parsed.description === 'string') {
				flowStore.val.description = parsed.description
			}
			if (parsed['ws_error_handler_muted'] !== undefined) {
				flowStore.val.ws_error_handler_muted = parsed['ws_error_handler_muted']
			}
			if (parsed['dedicated_worker'] !== undefined) {
				flowStore.val.dedicated_worker = parsed['dedicated_worker']
			}
			if (parsed['visible_to_runner_only'] !== undefined) {
				flowStore.val.visible_to_runner_only = parsed['visible_to_runner_only']
			}
			if (parsed['on_behalf_of_email'] !== undefined) {
				flowStore.val.on_behalf_of_email = parsed['on_behalf_of_email']
			}
			flowStore.val.value = parsed.value
			flowStore.val.schema = parsed.schema
			flowStore.val.tag = parsed.tag
			refreshStateStore(flowStore)
			initialCode = code
			sendUserToast('Changes applied')
		} catch (e) {
			;(sendUserToast('Error parsing yaml: ' + e), true)
		}
	}

	let editorHeight = $state(0)
</script>

<Drawer on:open={reload} bind:this={drawer} size="800px">
	<DrawerContent title="OpenFlow" on:close={() => drawer?.toggleDrawer()}>
		{#snippet actions()}
			<Button variant="default" unifiedSize="md" disabled={!hasChanges} on:click={reload}>Reset code</Button>
			<Button variant="accent" unifiedSize="md" disabled={!hasChanges} on:click={apply}>Apply changes</Button>
		{/snippet}

		{#if flowStore.val}
			{#await import('../../SimpleEditor.svelte')}
				<Loader2 class="animate-spin" />
			{:then Module}
				<div class="h-full w-full overflow-hidden" bind:clientHeight={editorHeight}>
					<Module.default
						bind:this={editor}
						autoHeight
						minHeight={editorHeight}
						bind:code
						lang="yaml"
					/>
				</div>
			{/await}
		{/if}
	</DrawerContent>
</Drawer>
