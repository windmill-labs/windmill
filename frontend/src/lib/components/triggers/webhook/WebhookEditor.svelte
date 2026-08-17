<script lang="ts">
	import { setContext, tick } from 'svelte'
	import { writable } from 'svelte/store'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import WebhooksPanel from './WebhooksPanel.svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import { Triggers } from '$lib/components/triggers/triggers.svelte'
	import type { TriggersCount } from '$lib/gen'

	// WebhooksPanel → TriggerTokens reads the 'TriggerContext' to publish the
	// webhook token count. The pipeline canvas isn't a trigger editor, so no
	// ancestor provides it — supply a standalone context here. Nothing
	// consumes the count in this view, but the store must exist.
	setContext<TriggerContext>('TriggerContext', {
		triggersCount: writable<TriggersCount | undefined>(undefined),
		simplifiedPoll: writable(false),
		showCaptureHint: writable(undefined),
		triggersState: new Triggers()
	})

	let open = $state(false)
	let drawer = $state<Drawer | undefined>(undefined)
	let scriptPath = $state('')
	let isFlow = $state(false)
	let runnableVersion = $state<string | undefined>(undefined)
	let args = $state<Record<string, any>>({})

	// Webhooks have no trigger row to create — every deployed runnable gets an
	// implicit endpoint. The drawer just surfaces the URLs + the
	// webhook-specific token creation flow, so there's no edit/new split.
	export async function openDrawer(
		path: string,
		is_flow = false,
		version?: string,
		runnableArgs: Record<string, any> = {}
	) {
		scriptPath = path
		isFlow = is_flow
		runnableVersion = version
		args = runnableArgs
		open = true
		await tick()
		drawer?.openDrawer()
	}

	let scopes = $derived(
		isFlow ? [`jobs:run:flows:${scriptPath}`] : [`jobs:run:scripts:${scriptPath}`]
	)
</script>

{#if open}
	<Drawer size="700px" bind:this={drawer} on:close={() => (open = false)}>
		<DrawerContent title={`Webhooks for ${scriptPath}`} on:close={() => drawer?.closeDrawer()}>
			<WebhooksPanel
				{isFlow}
				path={scriptPath}
				{runnableVersion}
				token=""
				{args}
				{scopes}
				newItem={false}
			/>
		</DrawerContent>
	</Drawer>
{/if}
