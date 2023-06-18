<script lang="ts">
	import * as Diff from 'diff'
	import { Button, Drawer, DrawerContent } from './common'
	import { Loader2 } from 'lucide-svelte'

	let diffViewer: Drawer
	let diffContent: string | undefined = undefined

	export let button: { text: string; onClick: () => void } | undefined = undefined
	export function openDrawer() {
		diffContent = undefined
		diffViewer.openDrawer()
	}

	export function setDiff(local: string, remote: string) {
		let finalString = ''
		for (const part of Diff.diffLines(local, remote)) {
			if (part.removed) {
				// print red if removed without newline
				finalString += `<span class="text-red-600">${part.value}</span>`
			} else if (part.added) {
				// print green if added
				finalString += `<span class="text-green-600">${part.value}</span>`
			} else {
				let lines = part.value.split('\n')

				if (lines.length > 12) {
					lines = lines.slice(0, 6)
					lines.push('...')
					lines = lines.concat(part.value.split('\n').slice(-6))
				}
				// print white if unchanged
				finalString += `${lines.join('\n')}`
			}
		}
		diffContent = finalString
	}
</script>

<Drawer bind:this={diffViewer} size="800px">
	<DrawerContent title="Diff" on:close={diffViewer.closeDrawer}>
		{#if diffContent == undefined}
			<Loader2 class="animate-spin" />
		{:else}
			<pre class="border bg-white p-2"><code>{@html diffContent}</code></pre>
			<div class="flex flex-row-reverse gap-2">
				<div class="text-red-600">Removed</div>
				<div class="text-green-600">Added</div></div
			>
		{/if}
		<svelte:fragment slot="actions">
			{#if button}
				<Button
					color="light"
					on:click={() => {
						button?.onClick()
						diffViewer.closeDrawer()
					}}>{button.text}</Button
				>
			{/if}
		</svelte:fragment>
	</DrawerContent>
</Drawer>
