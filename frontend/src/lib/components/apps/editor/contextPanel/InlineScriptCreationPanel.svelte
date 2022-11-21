<script lang="ts">
	import { Button } from '$lib/components/common'
	import { Preview } from '$lib/gen'
	import { DENO_INIT_CODE_CLEAR } from '$lib/script_helpers'
	import { emptySchema } from '$lib/utils'
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../../types'

	const { app } = getContext<AppEditorContext>('AppEditorContext')

	export let appPath: string

	function createScript() {
		const scriptPath = 'name'
		const path = `${appPath}/inline-script/${scriptPath}`
		const inlineScript = {
			content: DENO_INIT_CODE_CLEAR,
			language: Preview.language.DENO,
			path,
			schema: emptySchema()
		}

		if ($app.inlineScripts) {
			$app.inlineScripts[scriptPath] = inlineScript
		} else {
			$app.inlineScripts = {
				[scriptPath]: inlineScript
			}
		}
	}
</script>

<input value="" />
<Button on:click={createScript}>Create</Button>
