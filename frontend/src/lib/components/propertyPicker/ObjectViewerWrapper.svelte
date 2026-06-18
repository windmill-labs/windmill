<script lang="ts">
	import ObjectViewer from './ObjectViewer.svelte'
	import { createEventDispatcher, onMount, untrack } from 'svelte'

	interface Props {
		json: any
		level?: number
		currentPath?: string
		pureViewer?: boolean
		collapsed?: any
		rawKey?: boolean
		topBrackets?: boolean
		allowCopy?: boolean
		collapseLevel?: number | undefined
		prefix?: string
		expandedEvenOnLevel0?: string | undefined
		connecting?: boolean
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
		connecting = false
	}: Props = $props()

	const _props = {
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

	const dispatch = createEventDispatcher()

	onMount(() => {
		setTimeout(() => {
			dispatch('mounted') // Notify the parent when ready
		}, 0) // Allow DOM rendering before dispatching
	})
</script>

<div>
	<ObjectViewer {..._props} />
</div>
