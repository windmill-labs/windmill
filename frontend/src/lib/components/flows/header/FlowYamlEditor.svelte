<script lang="ts">
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import { filteredContentForExport } from '../utils'
	import YAML from 'yaml'

	import { Button } from '$lib/components/common'
	import { sendUserToast } from '$lib/toast'
	import { Loader2 } from 'lucide-svelte'

	const { flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	export let drawer: Drawer | undefined

	let code = ''

	function reload() {
		code = YAML.stringify(filteredContentForExport(flowStore.val))
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
			flowStore.val = flowStore.val
		} catch (e) {
			sendUserToast('Error parsing yaml: ' + e), true
		}
	}
</script>

<Drawer on:open={reload} bind:this={drawer} size="800px">
	<DrawerContent title="OpenFlow" on:close={() => drawer?.toggleDrawer()}>
		<svelte:fragment slot="actions">
			<Button color="dark" size="sm" on:click={reload}>Reset code</Button>
			<Button color="dark" size="sm" on:click={apply}>Apply changes</Button>
		</svelte:fragment>

		{#if flowStore.val}
			{#await import('../../SimpleEditor.svelte')}
				<Loader2 class="animate-spin" />
			{:then Module}
				<Module.default autoHeight bind:code lang="yaml" />
			{/await}
		{/if}
	</DrawerContent>
</Drawer>
