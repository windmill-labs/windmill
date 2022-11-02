<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import ToggleButton from '$lib/components/common/toggleButton/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton/ToggleButtonGroup.svelte'
	import {
		faArrowsLeftRight,
		faDisplay,
		faExternalLink,
		faHand
	} from '@fortawesome/free-solid-svg-icons'
	import type { EditorMode } from '../types'

	export let title: string
	export let mode: EditorMode
</script>

<div class="border-b h-12 flex flex-row justify-between py-2 px-4 items-center">
	<span class="text-sm">{title}</span>
	<div class="flex flex-row gap-2">
		<ToggleButtonGroup bind:selected={mode}>
			<ToggleButton position="left" value="dnd" startIcon={{ icon: faHand }}>
				Component editor
			</ToggleButton>
			<ToggleButton position="center" value="width" startIcon={{ icon: faArrowsLeftRight }}>
				Width editor
			</ToggleButton>
			<ToggleButton position="right" value="preview" startIcon={{ icon: faDisplay }}>
				Preview
			</ToggleButton>
		</ToggleButtonGroup>
		<Button color="dark" size="sm" variant="border" startIcon={{ icon: faExternalLink }}>
			Publish
		</Button>
		<Button size="sm">Save</Button>
	</div>
</div>
