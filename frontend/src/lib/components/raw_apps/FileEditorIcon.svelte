<script lang="ts" module>
	export const SUPPORTED_EXTENSIONS = ['tsx', 'json', 'ts', 'js', 'vue', 'css', 'svelte']
</script>

<script lang="ts">
	import TypeScript from '../common/languageIcons/TypeScript.svelte'
	import JavaScriptIcon from '../icons/JavaScriptIcon.svelte'
	import JsonIcon from '../icons/JsonIcon.svelte'
	import ReactIcon from '../icons/ReactIcon.svelte'
	import SvelteIcon from '../icons/SvelteIcon.svelte'
	import VueIcon from '../icons/VueIcon.svelte'

	interface Props {
		file: string
		size?: number
	}

	let { file, size = 16 }: Props = $props()

	const width = $derived(size.toString() + 'px')
	const height = $derived(size.toString() + 'px')
</script>

{#if file.endsWith('.tsx')}
	<ReactIcon {width} {height} />
{:else if file.endsWith('.json')}
	<JsonIcon {width} {height} />
{:else if file.endsWith('.ts')}
	<TypeScript width={size} height={size} />
{:else if file.endsWith('.js')}
	<JavaScriptIcon {width} {height} />
{:else if file.endsWith('.vue')}
	<VueIcon {width} {height} />
{:else if file.endsWith('.css')}
	<span class="text-blue-600 ml-0.5" style="font-size: 16px;">#</span>
{:else if file.endsWith('.svelte')}
	<SvelteIcon {width} {height} />
{/if}
