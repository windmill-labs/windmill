<script lang="ts">
	import type { AppTheme } from '../../types'
	import { resolveTheme } from './themeUtils'
	import { workspaceStore } from '$lib/stores'
	import { onMount } from 'svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'

	export let theme: AppTheme | undefined = undefined

	let code: string | undefined = undefined
	let cssEditor: SimpleEditor | undefined = undefined

	onMount(async () => {
		code = await resolveTheme(theme, $workspaceStore)
		cssEditor?.setCode(code)
	})
</script>

<slot />
<div class="relative h-full">
	<div class="absolute top-0 left-0 right-0 bottom-0 bg-gray-100 bg-opacity-50 z-50" />
	<SimpleEditor
		class="h-full"
		lang="css"
		{code}
		fixedOverflowWidgets={false}
		small
		automaticLayout
		bind:this={cssEditor}
		deno={false}
	/>
</div>
