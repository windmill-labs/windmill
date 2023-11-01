<script lang="ts">
	import { Alert, Button, Drawer, DrawerContent } from './common'
	import { Loader2 } from 'lucide-svelte'
	import DiffEditor from './DiffEditor.svelte'
	import { scriptLangToEditorLang } from '$lib/scripts'
	import { DraftService, type NewScript, type NewScriptWithDraft, type Script } from '$lib/gen'
	import Tabs from './common/tabs/Tabs.svelte'
	import Tab from './common/tabs/Tab.svelte'
	import { cloneDeep } from 'lodash'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { cleanScriptProperties, sendUserToast } from '$lib/utils'
	import { deepEqual } from 'fast-equals'
	import { workspaceStore } from '$lib/stores'
	import { goto } from '$app/navigation'

	let selected: 'content' | 'metadata' | undefined = undefined
	let toggleSelected: 'draft' | 'deployed' | undefined = undefined
	let diffViewer: Drawer

	let data:
		| {
				deployed:
					| {
							lang: string
							content: string
							metadata: string
					  }
					| undefined
				draft:
					| {
							lang: string
							content: string
							metadata: string
					  }
					| undefined
				current: {
					lang: string
					content: string
					metadata: string
				}
		  }
		| undefined = undefined
	let path: string | undefined = undefined
	let draftOnly = false
	export let loadScript: () => Promise<void>

	async function restoreDraft() {
		if (!path) {
			sendUserToast('Could not restore draft', true)
			return
		}
		diffViewer.closeDrawer()
		goto(`/scripts/edit/${path}`)
		loadScript()
	}

	async function discardDraft() {
		if (!path) {
			sendUserToast('Could not restore to last deployed', true)
			return
		}
		diffViewer.closeDrawer()
		if (data?.draft) {
			await DraftService.deleteDraft({
				workspace: $workspaceStore!,
				kind: 'script',
				path
			})
		}
		goto(`/scripts/edit/${path}`)
		loadScript()
	}

	export function openDrawer() {
		data = undefined
		selected = undefined
		toggleSelected = undefined
		draftOnly = false
		diffViewer.openDrawer()
	}

	function prepareScriptDiff(script: NewScript | Script) {
		const metadata = cloneDeep(cleanScriptProperties(script))
		const content = metadata['content']
		metadata['content'] = 'check content diff'
		return {
			lang: scriptLangToEditorLang(script.language),
			content,
			metadata: JSON.stringify(metadata, null, 2)
		}
	}

	export function setDiff(
		deployed: NewScriptWithDraft | Script,
		draft: NewScript,
		current: NewScript
	) {
		draftOnly = deployed.draft_only || false
		path = deployed['draft']?.['path'] || deployed.path
		data = {
			deployed: deployed ? prepareScriptDiff(deployed) : undefined,
			draft: draft ? prepareScriptDiff(draft) : undefined,
			current: prepareScriptDiff(current)
		}
	}

	function updateToggleSelected(data_: typeof data, draftOnly: boolean) {
		if (!data_) return
		if (data_?.deployed && !draftOnly) {
			toggleSelected = 'deployed'
		} else if (data_?.draft) {
			toggleSelected = 'draft'
		}
	}
	$: updateToggleSelected(data, draftOnly)

	function updateSelected(data_: typeof data, toggleSelected_: typeof toggleSelected) {
		if (!data_) return
		if (!toggleSelected_) return
		selected =
			data_[toggleSelected_]?.content !== data_.current.content
				? 'content'
				: data_[toggleSelected_]?.metadata !== data_.current.metadata
				? 'metadata'
				: undefined
	}

	$: updateSelected(data, toggleSelected)
</script>

<Drawer bind:this={diffViewer} size="1200px">
	<DrawerContent title="Diff" on:close={diffViewer.closeDrawer}>
		<div class="flex flex-col gap-4 h-full">
			<ToggleButtonGroup bind:selected={toggleSelected} class="shrink-0">
				{#if data?.deployed && !draftOnly}
					<ToggleButton value="deployed" label="Deployed <> Current" />
				{/if}
				{#if data?.draft}
					<ToggleButton value="draft" label="Latest saved draft <> Current" />
				{/if}
			</ToggleButtonGroup>
			{#if toggleSelected === 'draft'}
				<Button
					size="xs"
					color="light"
					wrapperClasses="self-start"
					on:click={restoreDraft}
					disabled={deepEqual(data?.draft, data?.current)}>Restore to latest saved draft</Button
				>
			{:else if toggleSelected === 'deployed'}
				<Button
					size="xs"
					color="light"
					wrapperClasses="self-start"
					on:click={discardDraft}
					disabled={deepEqual(data?.deployed, data?.current)}
				>
					{data?.draft ? 'Discard draft and restore' : 'Restore'} to deployed
				</Button>
			{/if}
			{#if data}
				{#if selected}
					{@const content =
						toggleSelected === 'draft'
							? data.draft?.content
							: toggleSelected === 'deployed'
							? data.deployed?.content
							: undefined}
					{@const metadata =
						toggleSelected === 'draft'
							? data.draft?.metadata
							: toggleSelected === 'deployed'
							? data.deployed?.metadata
							: undefined}
					{@const lang =
						toggleSelected === 'draft'
							? data.draft?.lang
							: toggleSelected === 'deployed'
							? data.deployed?.lang
							: undefined}
					<div class="flex flex-col h-full gap-4">
						<Tabs bind:selected>
							<Tab value="content" disabled={content === data.current.content}
								>Content{content === data.current.content ? ' (no changes)' : ''}</Tab
							>
							<Tab value="metadata" disabled={metadata === data.current.metadata}
								>Metadata{metadata === data.current.metadata ? ' (no changes)' : ''}</Tab
							>
						</Tabs>
						<div class="flex-1">
							{#key toggleSelected}
								{#if selected === 'content'}
									<DiffEditor
										automaticLayout
										class="h-full"
										defaultLang={lang}
										defaultModifiedLang={data.current.lang}
										defaultOriginal={content}
										defaultModified={data.current.content}
										readOnly
									/>
								{:else if selected === 'metadata'}
									<DiffEditor
										automaticLayout
										class="h-full"
										defaultLang="json"
										defaultOriginal={metadata}
										defaultModified={data.current.metadata}
										readOnly
									/>
								{/if}
							{/key}
						</div>
					</div>
				{:else}
					<Alert title="No changes detected">
						There are no differences between {toggleSelected === 'draft'
							? 'latest saved draft'
							: toggleSelected} and current
					</Alert>
				{/if}
			{:else}
				<Loader2 class="animate-spin" />
			{/if}
		</div>
	</DrawerContent>
</Drawer>
