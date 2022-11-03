<script lang="ts">
	import type { Schema } from '$lib/common'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import { emptySchema } from '$lib/utils'
	import { getContext } from 'svelte'
	import type { AppEditorContext, AppInputTransform } from '../types'

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')
	export let inputs: {
		result: AppInputTransform
	}

	export const schema: Schema = {
		$schema: 'https://json-schema.org/draft/2020-12/schema',
		properties: {
			result: {
				type: 'any',
				description: ''
			}
		},
		required: ['result'],
		type: 'object'
	}

	$: inputResult = $worldStore?.connect<any>(inputs.result, (x) => {
		update()
	})

	let result: any

	function update() {
		result = inputResult?.peak()
	}

	export const staticOutputs: string[] = []
</script>

{#if $worldStore}
	<DisplayResult {result} />
{/if}
