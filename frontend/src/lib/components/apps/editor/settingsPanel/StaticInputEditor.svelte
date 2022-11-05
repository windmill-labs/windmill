<script lang="ts">
	import type { AppEditorContext, StaticInput, TriggerablePolicy } from '../../types'
	import Toggle from '$lib/components/Toggle.svelte'
	import { getContext } from 'svelte'

	export let input: StaticInput
	export let propKey: string
	export let componenId: string

	const { app } = getContext<AppEditorContext>('AppEditorContext')

	let value = $app.policy.triggerables?.[componenId]?.staticFields?.[propKey]

	function updatePolicy() {
		$app.policy.triggerables[componenId].staticFields[propKey] = value
	}

	$: value && updatePolicy()
</script>

<Toggle bind:checked={input.visible} options={{ right: 'Visible' }} />
<input bind:value />
