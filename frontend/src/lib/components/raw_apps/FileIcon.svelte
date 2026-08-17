<!--
@component
Icon for a file (or folder) chosen from its extension. Single source of truth
for raw-app file icons — used by the file tree (FileTreeNode) and the fork-diff
rows (via RowIcon). Pass a bare name or a full path; the extension is taken from
the last path segment. Falls back to a generic file icon.
-->
<script lang="ts">
	import { File, Folder, ImageIcon } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import TypeScript from '../common/languageIcons/TypeScript.svelte'
	import JavaScriptIcon from '../icons/JavaScriptIcon.svelte'
	import JsonIcon from '../icons/JsonIcon.svelte'
	import ReactIcon from '../icons/ReactIcon.svelte'
	import SvelteIcon from '../icons/SvelteIcon.svelte'
	import VueIcon from '../icons/VueIcon.svelte'
	import CssIcon from '../icons/CssIcon.svelte'
	import SassIcon from '../icons/SassIcon.svelte'
	import LessIcon from '../icons/LessIcon.svelte'
	import HtmlIcon from '../icons/HtmlIcon.svelte'
	import MarkdownIcon from '../icons/MarkdownIcon.svelte'
	import YamlIcon from '../icons/YamlIcon.svelte'

	interface Props {
		/** File name or path; extension drives the icon. */
		name: string
		isFolder?: boolean
		size?: number
		class?: string
	}

	let { name, isFolder = false, size = 14, class: className = '' }: Props = $props()

	function extOf(n: string): string {
		const base = n.split('/').pop() ?? n
		return base.includes('.') ? (base.split('.').pop() ?? '').toLowerCase() : ''
	}

	const spec = $derived.by(() => {
		if (isFolder) return { icon: Folder, className: 'text-secondary' }
		switch (extOf(name)) {
			case 'json':
				return { icon: JsonIcon, className: '' }
			case 'tsx':
			case 'jsx':
				return { icon: ReactIcon, className: '' }
			case 'ts':
				return { icon: TypeScript, className: '' }
			case 'js':
				return { icon: JavaScriptIcon, className: '' }
			case 'svelte':
				return { icon: SvelteIcon, className: '' }
			case 'vue':
				return { icon: VueIcon, className: '' }
			case 'css':
				return { icon: CssIcon, className: '' }
			case 'scss':
			case 'sass':
				return { icon: SassIcon, className: '' }
			case 'less':
				return { icon: LessIcon, className: '' }
			case 'png':
			case 'jpg':
			case 'jpeg':
			case 'gif':
			case 'svg':
			case 'webp':
			case 'ico':
				return { icon: ImageIcon, className: 'text-purple-500' }
			case 'html':
			case 'htm':
				return { icon: HtmlIcon, className: '' }
			case 'md':
			case 'markdown':
				return { icon: MarkdownIcon, className: '' }
			case 'yaml':
			case 'yml':
				return { icon: YamlIcon, className: '' }
			default:
				return { icon: File, className: 'text-tertiary' }
		}
	})
</script>

{#if spec}
	{@const Icon = spec.icon}
	<Icon {size} class={twMerge('flex-shrink-0', spec.className, className)} />
{/if}
