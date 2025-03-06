<script lang="ts">
	import ObjectViewer from './ObjectViewer.svelte'
	import { createEventDispatcher, onMount } from 'svelte'

	export let json: any
	export let level = 0
	export let currentPath: string = ''
	export let pureViewer = false
	export let collapsed = (level != 0 && level % 3 == 0) || Array.isArray(json)
	export let rawKey = false
	export let topBrackets = false
	export let allowCopy = true
	export let collapseLevel: number | undefined = undefined
	export let prefix = ''
	export let expandedEvenOnLevel0: string | undefined = undefined
	export let connecting = false

	const props = {
		json,
		level,
		currentPath,
		pureViewer,
		collapsed,
		rawKey,
		topBrackets,
		allowCopy,
		collapseLevel,
		prefix,
		expandedEvenOnLevel0,
		connecting
	}

	const dispatch = createEventDispatcher()

	onMount(() => {
		setTimeout(() => {
			dispatch('mounted') // Notify the parent when ready
		}, 0) // Allow DOM rendering before dispatching
	})
</script>

<div>
	<ObjectViewer {...props} />
</div>
