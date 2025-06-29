<script lang="ts">
	import type { AppTheme } from '../../types'
	import { resolveTheme } from './themeUtils'
	import { workspaceStore } from '$lib/stores'
	import { onMount } from 'svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'

	interface Props {
		theme?: AppTheme | undefined
		children?: import('svelte').Snippet
	}

	let { theme = undefined, children }: Props = $props()

	let code: string | undefined = $state(undefined)
	let cssEditor: SimpleEditor | undefined = $state(undefined)

	onMount(async () => {
		code = await resolveTheme(theme, $workspaceStore)
		cssEditor?.setCode(code)
	})
</script>

<div class="relative h-full">
	<div class="absolute z-[100] left-[50%] top-[50%] translate-x-[-50%] translate-y-[-50%]">
		{@render children?.()}
	</div>
	<div class="absolute top-0 left-0 right-0 bottom-0 bg-gray-100 bg-opacity-50 z-50"></div>
	<SimpleEditor
		class="h-full"
		lang="css"
		{code}
		fixedOverflowWidgets={false}
		small
		automaticLayout
		bind:this={cssEditor}
	/>
</div>
