<script lang="ts">
	import NativeTriggerEditor from './NativeTriggerEditor.svelte'
	import Description from '$lib/components/Description.svelte'
	import { onMount, type Snippet } from 'svelte'
	import type { NativeServiceName } from '$lib/gen'
	import type { Trigger } from '../utils'
	import { NATIVE_TRIGGER_SERVICES } from './utils'

	interface Props {
		service: NativeServiceName
		selectedTrigger: Trigger
		isFlow: boolean
		path: string
		isDeployed?: boolean
		defaultValues?: Record<string, any>
		customLabel?: Snippet
	}

	let {
		service,
		selectedTrigger,
		isFlow,
		path,
		isDeployed = false,
		defaultValues = undefined,
		customLabel = undefined,
		...restProps
	}: Props = $props()

	let editor: NativeTriggerEditor | undefined = $state(undefined)

	async function openEditor(isFlow: boolean, isDraft: boolean) {
		if (isDraft) {
			editor?.openNew(isFlow, path, defaultValues)
		} else {
			// selectedTrigger.path contains the external_id for native triggers
			editor?.openEdit(selectedTrigger.path!, isFlow, defaultValues)
		}
	}

	onMount(() => {
		openEditor(isFlow, selectedTrigger.isDraft ?? false)
	})

	const serviceDisplayName = $derived(NATIVE_TRIGGER_SERVICES[service].serviceDisplayName)
</script>

<div class="flex flex-col gap-4">
	<NativeTriggerEditor
		bind:this={editor}
		{service}
		useDrawer={false}
		hideTarget
		allowDraft={true}
		trigger={selectedTrigger}
		{customLabel}
		{isDeployed}
		{...restProps}
	>
		{#snippet description()}
			<Description link="https://www.windmill.dev/docs/core_concepts/{service}_triggers">
				{serviceDisplayName} triggers execute scripts and flows in response to events in {serviceDisplayName}.
			</Description>
		{/snippet}
	</NativeTriggerEditor>
</div>
