<script lang="ts">
	import { Drawer, DrawerContent } from '$lib/components/common'
	import { Highlight } from 'svelte-highlight'
	import type { AppTheme } from '../../types'
	import css from 'svelte-highlight/languages/css'
	import { resolveTheme } from './themeUtils'
	import { workspaceStore } from '$lib/stores'
	import HighlightTheme from '$lib/components/HighlightTheme.svelte'

	interface Props {
		theme: AppTheme
	}

	let { theme }: Props = $props()

	let code: string | undefined = $state(undefined)
	let codeDrawer: Drawer | undefined = $state()

	export async function openDrawer() {
		codeDrawer?.openDrawer()
		code = await resolveTheme(theme, $workspaceStore)
	}
</script>

<HighlightTheme />

<Drawer bind:this={codeDrawer}>
	<DrawerContent title="Theme viewer" on:close={codeDrawer.closeDrawer}>
		<div class="p-2 border rounded-sm">
			<Highlight code={code ?? ''} language={css} />
		</div>
	</DrawerContent>
</Drawer>
