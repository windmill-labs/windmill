<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'
	import PanelSection from './common/PanelSection.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import MultiSelect from '$lib/components/multiselect/MultiSelect.svelte'

	export let recomputeIds: string[] | undefined = undefined
	export let ownId: string
	export let title: string = 'Trigger runnables on success'
	export let tooltip: string =
		'Select components to recompute after this runnable has successfully run'
	export let documentationLink: string =
		'https://www.windmill.dev/docs/apps/app-runnable-panel#trigger-runnables-on-success'

	const { runnableComponents } = getContext<AppViewerContext>('AppViewerContext')

	function onChange(checked: boolean, id: string) {
		if (checked) {
			recomputeIds = [...(recomputeIds ?? []), id]
		} else {
			recomputeIds = recomputeIds?.filter((x) => x !== id)
		}
	}
</script>

<PanelSection {title} {tooltip} {documentationLink}>
	<MultiSelect
		inputClass="!text-sm !h-8"
		liSelectedClass="!text-sm"
		options={Object.keys($runnableComponents ?? {})}
	/>
</PanelSection>
