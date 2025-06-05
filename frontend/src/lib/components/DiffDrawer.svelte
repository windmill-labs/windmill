<script lang="ts">
	import { Alert, Button, Drawer, DrawerContent } from './common'
	import { Loader2 } from 'lucide-svelte'
	import { scriptLangToEditorLang } from '$lib/scripts'
	import Tabs from './common/tabs/Tabs.svelte'
	import Tab from './common/tabs/Tab.svelte'
	import {
		cleanValueProperties,
		orderedJsonStringify,
		orderedYamlStringify,
		replaceFalseWithUndefined,
		type Value
	} from '$lib/utils'
	import type { Script } from '$lib/gen'

	type DiffData = {
		lang?: string
		content?: string
		metadata: string
	}

	let contentType: 'content' | 'metadata' | undefined = undefined
	let diffType: 'draft' | 'deployed' | 'custom' | undefined = undefined
	let diffViewer: Drawer

	export let restoreDeployed: () => Promise<void> = async () => {}
	export let restoreDraft: () => Promise<void> = async () => {}

	let data:
		| {
				mode: 'normal'
				deployed: DiffData | undefined
				draft: DiffData | undefined
				current: DiffData
				path?: string
				button?: { text: string; onClick: () => void }
		  }
		| {
				mode: 'simple'
				title: string
				original: DiffData | undefined
				current: DiffData
				button?: { text: string; onClick: () => void }
		  }
		| undefined = undefined

	export function openDrawer() {
		data = undefined
		contentType = undefined
		diffType = undefined
		diffViewer.openDrawer()
	}

	export function closeDrawer() {
		diffViewer.closeDrawer()
	}

	function prepareDiff(data: Value) {
		const metadata = structuredClone(cleanValueProperties(replaceFalseWithUndefined(data)))
		const content = metadata['content']
		if (metadata['content'] !== undefined) {
			metadata['content'] = 'check content diff'
		}
		return {
			lang: data.language ? scriptLangToEditorLang(data.language as Script['language']) : undefined,
			content,
			metadata: orderedYamlStringify(metadata)
		}
	}

	export function setDiff(
		diff:
			| {
					mode: 'normal'
					deployed: Value
					draft: Value | undefined
					current: Value
					defaultDiffType?: 'deployed' | 'draft'
					button?: { text: string; onClick: () => void }
			  }
			| {
					mode: 'simple'
					original: Value
					current: Value
					title: string
					button?: { text: string; onClick: () => void }
			  }
	) {
		if (diff.mode === 'normal') {
			const { deployed, draft, current, defaultDiffType, button } = diff
			data = {
				mode: 'normal',
				deployed: !deployed.draft_only ? prepareDiff(deployed) : undefined,
				draft: draft ? prepareDiff(draft) : undefined,
				current: prepareDiff(current),
				path: draft?.path || deployed?.path,
				button
			}

			if (defaultDiffType && data[defaultDiffType]) {
				diffType = defaultDiffType
			} else if (data.deployed) {
				diffType = 'deployed'
			} else if (data.draft) {
				diffType = 'draft'
			}
		} else {
			const { original, current, title, button } = diff
			data = {
				title,
				mode: 'simple',
				original: prepareDiff(original),
				current: prepareDiff(current),
				button
			}
			diffType = 'custom'
		}
	}

	function updateContentType(data_: typeof data, diffType_: typeof diffType) {
		if (!data_) return
		if (!diffType_) return
		const dataType = diffType_ === 'custom' ? 'original' : diffType_
		contentType =
			data_[dataType]?.content !== data_.current.content
				? 'content'
				: data_[dataType]?.metadata !== data_.current.metadata
					? 'metadata'
					: undefined
	}

	$: updateContentType(data, diffType)
</script>

<Drawer bind:this={diffViewer} size="1200px" on:close>
	<DrawerContent title="Diff" on:close={diffViewer.closeDrawer}>
		<div class="flex flex-col gap-4 h-full">
			{#if diffType && data}
				<Tabs bind:selected={diffType} wrapperClass="shrink-0">
					{#if data.mode === 'simple'}
						<Tab value="custom">{data.title}</Tab>
					{:else}
						<Tab value="deployed" disabled={!data.deployed}
							>{'Deployed <> Current'}{!data.deployed ? ' (no deployed version)' : ''}</Tab
						>
						<Tab value="draft" disabled={!data.draft}
							>{'Latest saved draft <> Current'}{!data.draft ? ' (no draft)' : ''}</Tab
						>
					{/if}
				</Tabs>
			{/if}
			{#if data?.mode === 'normal'}
				{#if diffType === 'draft'}
					<Button
						size="xs"
						color="light"
						variant="border"
						wrapperClasses="self-start"
						on:click={restoreDraft}
						disabled={orderedJsonStringify(data.draft) === orderedJsonStringify(data.current)}
						>Restore to latest saved draft</Button
					>
				{:else if diffType === 'deployed'}
					<Button
						size="xs"
						color="light"
						variant="border"
						wrapperClasses="self-start"
						on:click={restoreDeployed}
						disabled={!data.draft &&
							orderedJsonStringify(data.deployed) === orderedJsonStringify(data.current)}
					>
						Restore to deployed{data.draft ? ' and discard draft' : ''}
					</Button>
				{/if}
			{/if}
			{#if data}
				{#if contentType}
					{@const content =
						data.mode === 'normal'
							? diffType === 'draft'
								? data.draft?.content
								: data.deployed?.content
							: data.original?.content}
					{@const metadata =
						data.mode === 'normal'
							? diffType === 'draft'
								? data.draft?.metadata
								: data.deployed?.metadata
							: data.original?.metadata}
					{@const lang =
						data.mode === 'normal'
							? diffType === 'draft'
								? data.draft?.lang
								: data.deployed?.lang
							: data.original?.lang}
					<div class="flex flex-col h-full gap-4">
						{#if data.current.content !== undefined}
							<Tabs bind:selected={contentType}>
								<Tab value="content" disabled={content === data.current.content}
									>Content{content === data.current.content ? ' (no changes)' : ''}</Tab
								>
								<Tab value="metadata" disabled={metadata === data.current.metadata}
									>Metadata{metadata === data.current.metadata ? ' (no changes)' : ''}</Tab
								>
							</Tabs>
						{/if}
						<div class="flex-1">
							{#key diffType}
								{#if contentType === 'content'}
									{#await import('$lib/components/DiffEditor.svelte')}
										<Loader2 class="animate-spin" />
									{:then Module}
										<Module.default
											open={true}
											automaticLayout
											class="h-full"
											defaultLang={lang}
											defaultModifiedLang={data.current.lang}
											defaultOriginal={content}
											defaultModified={data.current.content}
											readOnly
										/>
									{/await}
								{:else if contentType === 'metadata'}
									{#await import('$lib/components/DiffEditor.svelte')}
										<Loader2 class="animate-spin" />
									{:then Module}
										<Module.default
											open={true}
											automaticLayout
											class="h-full"
											defaultLang="yaml"
											defaultOriginal={metadata}
											defaultModified={data.current.metadata}
											readOnly
										/>
									{/await}
								{/if}
							{/key}
						</div>
					</div>
				{:else}
					<Alert title="No changes detected">
						{#if diffType === 'draft'}
							There are no differences between latest saved draft and current
						{:else if diffType === 'deployed'}
							There are no differences between deployed and current
						{:else if diffType === 'custom'}
							There are no differences
						{/if}
					</Alert>
				{/if}
			{:else}
				<Loader2 class="animate-spin" />
			{/if}
		</div>
		<svelte:fragment slot="actions">
			{#if data?.button}
				<Button
					color="light"
					on:click={() => {
						if (data?.button) {
							data.button.onClick()
							diffViewer.closeDrawer()
						}
					}}>{data.button.text}</Button
				>
			{/if}
		</svelte:fragment>
	</DrawerContent>
</Drawer>
