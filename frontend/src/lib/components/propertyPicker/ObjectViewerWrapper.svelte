<script lang="ts">
	import ObjectViewer from './ObjectViewer.svelte'
	import { onMount, untrack } from 'svelte'

	interface Props {
		json: any
		level?: number
		currentPath?: string
		pureViewer?: boolean
		collapsed?: boolean
		rawKey?: boolean
		topBrackets?: boolean
		allowCopy?: boolean
		collapseLevel?: number | undefined
		prefix?: string
		expandedEvenOnLevel0?: string | undefined
		connecting?: boolean
		onmounted?: () => void
	}

	let {
		json,
		level = 0,
		currentPath = '',
		pureViewer = false,
		collapsed = (level != 0 && level % 3 == 0) || Array.isArray(json),
		rawKey = false,
		topBrackets = false,
		allowCopy = true,
		collapseLevel = undefined,
		prefix = '',
		expandedEvenOnLevel0 = undefined,
		connecting = false,
		onmounted = undefined
	}: Props = $props()

	const viewerProps = {
		json: untrack(() => json),
		level: untrack(() => level),
		currentPath: untrack(() => currentPath),
		pureViewer: untrack(() => pureViewer),
		collapsed: untrack(() => collapsed),
		rawKey: untrack(() => rawKey),
		topBrackets: untrack(() => topBrackets),
		allowCopy: untrack(() => allowCopy),
		collapseLevel: untrack(() => collapseLevel),
		prefix: untrack(() => prefix),
		expandedEvenOnLevel0: untrack(() => expandedEvenOnLevel0),
		connecting: untrack(() => connecting)
	}

	onMount(() => {
		setTimeout(() => {
			onmounted?.()
		}, 0)
	})
</script>

<div>
	<ObjectViewer {...viewerProps} />
</div>
