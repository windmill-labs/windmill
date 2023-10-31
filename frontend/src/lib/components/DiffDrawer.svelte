<script lang="ts">
	import { Alert, Button, Drawer, DrawerContent } from './common'
	import { Loader2 } from 'lucide-svelte'
	import DiffEditor from './DiffEditor.svelte'
	import { scriptLangToEditorLang } from '$lib/scripts'
	import type { Script } from '$lib/gen'
	import Tabs from './common/tabs/Tabs.svelte'
	import Tab from './common/tabs/Tab.svelte'
	import { cloneDeep } from 'lodash'

	let selected: 'content' | 'metadata' | undefined = undefined
	let diffViewer: Drawer
	let originalContent: string | undefined = undefined
	let originalMetadata: string | undefined = undefined
	let originalLang: string | undefined = undefined
	let modifiedContent: string | undefined = undefined
	let modifiedMetadata: string | undefined = undefined
	let modifiedLang: string | undefined = undefined

	export let button: { text: string; onClick: () => void } | undefined = undefined
	export function openDrawer() {
		originalContent = undefined
		originalMetadata = undefined
		originalLang = undefined
		modifiedContent = undefined
		modifiedMetadata = undefined
		modifiedLang = undefined
		diffViewer.openDrawer()
	}

	export function setDiff(
		local:
			| {
					content?: string
					language?: Script.language
					[key: string]: any
			  }
			| undefined,
		remote:
			| {
					content?: string
					language?: Script.language
					[key: string]: any
			  }
			| undefined
	) {
		if (local) {
			const local_ = cloneDeep(local)
			if (local_.content && local_.language) {
				originalContent = local_.content
				originalLang = scriptLangToEditorLang(local_.language)
				local_.content = 'check content diff'
			}
			originalMetadata = JSON.stringify(local_, null, 2)
		}
		if (remote) {
			const remote_ = cloneDeep(remote)
			if (remote_.content && remote_.language) {
				modifiedContent = remote_.content
				modifiedLang = scriptLangToEditorLang(remote_.language)
				remote_.content = 'check content diff'
			}
			modifiedMetadata = JSON.stringify(remote_, null, 2)
		}
		selected =
			originalContent !== modifiedContent
				? 'content'
				: originalMetadata !== modifiedMetadata
				? 'metadata'
				: undefined
	}
</script>

<Drawer bind:this={diffViewer} size="1200px">
	<DrawerContent title="Diff" on:close={diffViewer.closeDrawer}>
		{#if originalMetadata !== undefined && originalMetadata !== undefined}
			{#if selected}
				<div class="flex flex-col h-full gap-4">
					<Tabs bind:selected>
						{#if originalContent !== undefined && originalContent !== modifiedContent}
							<Tab value="content">Content</Tab>
						{/if}
						{#if originalMetadata !== modifiedMetadata}
							<Tab value="metadata">Metadata</Tab>
						{/if}
					</Tabs>
					<div class="flex-1">
						{#if selected === 'content'}
							<DiffEditor
								automaticLayout
								class="h-full"
								defaultLang={originalLang}
								defaultModifiedLang={modifiedLang}
								defaultOriginal={originalContent}
								defaultModified={modifiedContent}
								readOnly
							/>
						{:else if selected === 'metadata'}
							<DiffEditor
								automaticLayout
								class="h-full"
								defaultLang="json"
								defaultOriginal={originalMetadata}
								defaultModified={modifiedMetadata}
								readOnly
							/>
						{/if}</div
					>
				</div>
			{:else}
				<Alert title="No changes detected">
					There are no differences between the local and remote content
				</Alert>
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
