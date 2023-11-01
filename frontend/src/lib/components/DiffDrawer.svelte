<script lang="ts">
	import { Alert, Button, Drawer, DrawerContent } from './common'
	import { Loader2 } from 'lucide-svelte'
	import DiffEditor from './DiffEditor.svelte'
	import { scriptLangToEditorLang } from '$lib/scripts'
	import type { Script } from '$lib/gen'
	import Tabs from './common/tabs/Tabs.svelte'
	import Tab from './common/tabs/Tab.svelte'
	import { cloneDeep } from 'lodash'
	import { cleanScriptProperties } from '$lib/utils'

	export let title = 'Diff'

	let selected: 'content' | 'metadata' | undefined = undefined
	let diffViewer: Drawer
	let data:
		| {
				original: {
					lang?: string
					content?: string
					metadata: string
				}
				current: {
					lang?: string
					content?: string
					metadata: string
				}
		  }
		| undefined = undefined

	export let button: { text: string; onClick: () => void } | undefined = undefined
	export function openDrawer() {
		data = undefined
		selected = undefined
		title = 'Diff'
		diffViewer.openDrawer()
	}

	function prepareDiff(data: { content?: string; language?: string; [key: string]: any }) {
		const metadata = cloneDeep(
			data.content !== undefined && data.language !== undefined ? cleanScriptProperties(data) : data
		)
		const content = metadata['content']
		if (metadata['content'] !== undefined) {
			metadata['content'] = 'check content diff'
		}
		return {
			lang: data.language ? scriptLangToEditorLang(data.language as Script.language) : undefined,
			content,
			metadata: JSON.stringify(metadata, null, 2)
		}
	}

	export function setDiff(
		original: {
			content?: string
			language?: Script.language
			[key: string]: any
		},
		current: {
			content?: string
			language?: Script.language
			[key: string]: any
		},
		newTitle?: string
	) {
		data = {
			original: prepareDiff(original),
			current: prepareDiff(current)
		}
		selected =
			data.original.content !== data.current.content
				? 'content'
				: data.original.metadata !== data.current.metadata
				? 'metadata'
				: undefined
		title = newTitle || title
	}
</script>

<Drawer bind:this={diffViewer} size="1200px">
	<DrawerContent {title} on:close={diffViewer.closeDrawer}>
		{#if data !== undefined}
			{#if selected}
				<div class="flex flex-col h-full gap-4">
					{#if data.original.content !== undefined}
						<Tabs bind:selected>
							<Tab value="content" disabled={data.original.content === data.current.content}
								>Content{data.original.content === data.current.content ? ' (no changes)' : ''}</Tab
							>
							<Tab value="metadata" disabled={data.original.metadata === data.current.metadata}
								>Metadata{data.original.metadata === data.current.metadata
									? ' (no changes)'
									: ''}</Tab
							>
						</Tabs>
					{/if}
					<div class="flex-1">
						{#if selected === 'content'}
							<DiffEditor
								automaticLayout
								class="h-full"
								defaultLang={data.original.lang}
								defaultModifiedLang={data.current.lang}
								defaultOriginal={data.original.content}
								defaultModified={data.current.content}
								readOnly
							/>
						{:else if selected === 'metadata'}
							<DiffEditor
								automaticLayout
								class="h-full"
								defaultLang="json"
								defaultOriginal={data.original.metadata}
								defaultModified={data.current.metadata}
								readOnly
							/>
						{/if}</div
					>
				</div>
			{:else}
				<Alert title="No changes detected">There are no differences</Alert>
			{/if}
		{:else}
			<Loader2 class="animate-spin" />
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
