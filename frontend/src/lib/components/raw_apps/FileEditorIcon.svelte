<script lang="ts">
	import TypeScript from '../common/languageIcons/TypeScript.svelte'
	import JavaScriptIcon from '../icons/JavaScriptIcon.svelte'
	import JsonIcon from '../icons/JsonIcon.svelte'
	import ReactIcon from '../icons/ReactIcon.svelte'
	import SvelteIcon from '../icons/SvelteIcon.svelte'
	import VueIcon from '../icons/VueIcon.svelte'

	export let file: string
</script>

{#if file.endsWith('.tsx')}
	<ReactIcon width="16px" height="16px" />
{:else if file.endsWith('.json')}
	<JsonIcon width="16px" height="16px" />
{:else if file.endsWith('.ts')}
	<TypeScript width={16} height={16} />
{:else if file.endsWith('.js')}
	<JavaScriptIcon width="16px" height="16px" />
{:else if file.endsWith('.vue')}
	<VueIcon width="16px" height="16px" />
{:else if file.endsWith('.css')}
	<span class="text-blue-600 ml-0.5" style="font-size: 16px;">#</span>
{:else if file.endsWith('.svelte')}
	<SvelteIcon width="16px" height="16px" />
{/if}
