<script lang="ts">
	import { Highlight } from 'svelte-highlight'
	import type { AppTheme } from '../../types'
	import css from 'svelte-highlight/languages/css'
	import { resolveTheme } from './themeUtils'
	import { workspaceStore } from '$lib/stores'
	import { onMount } from 'svelte'

	export let theme: AppTheme | undefined = undefined

	let code: string | undefined = undefined

	onMount(async () => {
		code = await resolveTheme(theme, $workspaceStore)
	})
</script>

<div class="p-2 border m-2 rounded-sm">
	<Highlight code={code ?? ''} language={css} />
</div>
