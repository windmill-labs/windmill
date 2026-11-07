<script lang="ts">
	import { untrack } from 'svelte'
	import { UnresolvedFile, type FileContents } from '@pierre/diffs'
	import { useIsDarkMode } from '$lib/components/DarkModeObserver.svelte'
	import { PIERRE_THEME, pierreLang } from './pierre'

	interface Props {
		/** Text with git conflict markers. */
		contents: string
		filename?: string
		lang?: string
		onResolved?: (contents: string) => void
		class?: string
	}

	let { contents, filename = 'file', lang, onResolved, class: className = '' }: Props = $props()

	const isDark = useIsDarkMode()
	let container: HTMLDivElement | undefined = $state()
	let instance: UnresolvedFile | undefined = $state()

	const file: FileContents = $derived({ name: filename, contents, lang: pierreLang(lang) })

	$effect(() => {
		if (!container) return
		const u = untrack(
			() =>
				new UnresolvedFile({
					theme: PIERRE_THEME,
					themeType: isDark.val ? 'dark' : 'light',
					// Uncontrolled: the component applies each resolution itself.
					onMergeConflictResolve: (resolved) => onResolved?.(resolved.contents)
				})
		)
		instance = u
		return () => {
			u.cleanUp()
			instance = undefined
		}
	})

	$effect(() => {
		const themeType = isDark.val ? 'dark' : 'light'
		untrack(() => instance?.setThemeType(themeType))
	})

	$effect(() => {
		if (!instance || !container) return
		instance.render({ file, containerWrapper: container })
	})
</script>

<div bind:this={container} class={className}></div>
