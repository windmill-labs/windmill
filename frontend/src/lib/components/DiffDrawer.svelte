<script lang="ts">
	import { Alert, Button, Drawer, DrawerContent } from './common'
	import { ArrowRight, Loader2 } from 'lucide-svelte'
	import { scriptLangToEditorLang } from '$lib/scripts'
	import Tabs from './common/tabs/Tabs.svelte'
	import Tab from './common/tabs/Tab.svelte'
	import {
		cleanValueProperties,
		orderedJsonStringify,
		replaceFalseWithUndefined,
		type Value
	} from '$lib/utils'
	import { orderedYamlStringify } from '$lib/utils/orderedYaml'
	import type { Script } from '$lib/gen'
	import Select from './select/Select.svelte'
	import type { DiffVersionOption } from './diff_drawer'

	type DiffData = {
		lang?: string
		content?: string
		metadata: string
	}

	let diffType: 'draft' | 'deployed' | 'custom' | undefined = $state(undefined)

	let contentType = $derived.by(() => {
		if (!data || !diffType) return undefined
		const dataType = diffType === 'custom' ? 'original' : diffType
		return data[dataType]?.content !== data.current.content
			? 'content'
			: data[dataType]?.metadata !== data.current.metadata
				? 'metadata'
				: undefined
	})

	let diffViewer: Drawer | undefined = $state(undefined)

	interface Props {
		restoreDeployed?: () => Promise<void>
		isFlow?: boolean
	}

	let { restoreDeployed = undefined, isFlow = false }: Props = $props()

	let data:
		| {
				mode: 'normal'
				deployed: DiffData | undefined
				/** Which deployed version the left side is — the reader otherwise has no
				 *  way to tell what their draft is being compared against. */
				deployedLabel?: string
				versions?: DiffVersionOption[]
				onTakeLatest?: () => void | Promise<void>
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
		| undefined = $state(undefined)

	export function openDrawer() {
		data = undefined
		diffType = undefined
		diffViewer?.openDrawer()
	}

	export function closeDrawer() {
		diffViewer?.closeDrawer()
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

	// Which deployed version the left pane shows. The drawer keeps the id opaque and
	// hands it back to the editor, which knows how to fetch it for its own kind.
	let selectedVersion: string | undefined = $state(undefined)
	let versionLoader: ((id: string) => Promise<Value | undefined>) | undefined = $state(undefined)
	let headLabel: string | undefined = $state(undefined)
	let loadingVersion = $state(false)

	async function selectVersion(id: string | undefined) {
		if (!id || !versionLoader || !data || data.mode !== 'normal') return
		selectedVersion = id
		loadingVersion = true
		try {
			const value = await versionLoader(id)
			if (!value || !data || data.mode !== 'normal') return
			const opt = data.versions?.find((v) => v.id === id)
			data = {
				...data,
				deployed: prepareDiff(value),
				deployedLabel: opt?.isHead ? headLabel : opt?.label
			}
		} finally {
			loadingVersion = false
		}
	}

	let takingLatest = $state(false)
	async function takeLatest() {
		if (!data || data.mode !== 'normal' || !data.onTakeLatest || takingLatest) return
		takingLatest = true
		try {
			await data.onTakeLatest()
			diffViewer?.closeDrawer()
		} finally {
			takingLatest = false
		}
	}

	export function setDiff(
		diff:
			| {
					mode: 'normal'
					deployed: Value
					deployedLabel?: string
					versions?: DiffVersionOption[]
					loadVersion?: (id: string) => Promise<Value | undefined>
					onTakeLatest?: () => void | Promise<void>
					draft?: Value | undefined
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
			const {
				deployed,
				deployedLabel,
				versions,
				loadVersion,
				onTakeLatest,
				draft,
				current,
				button
			} = diff
			versionLoader = loadVersion
			headLabel = deployedLabel
			selectedVersion = versions?.find((v) => v.isHead)?.id
			data = {
				mode: 'normal',
				deployed: !deployed.draft_only ? prepareDiff(deployed) : undefined,
				deployedLabel,
				versions,
				onTakeLatest,
				draft: draft ? prepareDiff(draft) : undefined,
				current: prepareDiff(current),
				path: draft?.path || deployed?.path,
				button
			}

			// The draft-vs-current view (and its tab) is obsolete — always show the
			// deployed-vs-current diff.
			diffType = 'deployed'
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
</script>

<Drawer bind:this={diffViewer} size="1200px" on:close>
	<DrawerContent title="Diff" on:close={diffViewer.closeDrawer}>
		<div class="flex flex-col gap-4 h-full">
			{#if data}
				<!-- Outside the `contentType` check on purpose: with no differences against
				     the head, picking an older version is exactly how the reader finds one,
				     so the picker has to outlive the "no changes" state. -->
				{#if data.mode === 'normal' && (data.deployedLabel || data.versions?.length)}
					<div class="flex gap-2 items-center text-xs text-secondary">
						{#if data.versions?.length && versionLoader}
							<div class="w-72">
								<Select
									items={data.versions.map((v) => ({
										value: v.id,
										label: v.label,
										subtitle: v.subtitle
									}))}
									bind:value={() => selectedVersion, (v) => selectVersion(v as string)}
									disabled={loadingVersion}
									clearable={false}
								/>
							</div>
							{#if loadingVersion}
								<Loader2 size={12} class="animate-spin shrink-0" />
							{/if}
						{:else if data.deployedLabel}
							<span class="font-medium text-primary">{data.deployedLabel}</span>
						{/if}
						<ArrowRight size={12} class="shrink-0" />
						<span>your draft</span>
					</div>
				{/if}
				{#if contentType}
					{@const content =
						data.mode === 'normal' ? data.deployed?.content : data.original?.content}
					{@const metadata =
						data.mode === 'normal' ? data.deployed?.metadata : data.original?.metadata}
					{@const lang = data.mode === 'normal' ? data.deployed?.lang : data.original?.lang}
					<div class="flex flex-col h-full gap-4">
						{#if data.current.content !== undefined}
							<Tabs bind:selected={contentType}>
								<Tab
									value="content"
									disabled={content === data.current.content}
									label={`Content${content === data.current.content ? ' (no changes)' : ''}`}
								/>
								<Tab
									value="metadata"
									disabled={metadata === data.current.metadata}
									label={`Metadata${metadata === data.current.metadata ? ' (no changes)' : ''}`}
								/>
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
											className="h-full"
											defaultLang={lang}
											defaultModifiedLang={data.current.lang}
											defaultOriginal={content}
											defaultModified={data.current.content}
											readOnly
										/>
									{/await}
								{:else if contentType === 'metadata'}
									{#if isFlow}
										{#await import('$lib/components/FlowDiffViewer.svelte')}
											<Loader2 class="animate-spin" />
										{:then Module}
											<Module.default
												beforeYaml={metadata ?? ''}
												afterYaml={data.current.metadata}
											/>
										{/await}
									{:else}
										{#await import('$lib/components/DiffEditor.svelte')}
											<Loader2 class="animate-spin" />
										{:then Module}
											<Module.default
												open={true}
												automaticLayout
												className="h-full"
												defaultLang="yaml"
												defaultOriginal={metadata}
												defaultModified={data.current.metadata}
												readOnly
											/>
										{/await}
									{/if}
								{/if}
							{/key}
						</div>
					</div>
				{:else}
					<Alert title="No changes detected">
						{#if diffType === 'deployed'}
							There are no differences between deployed and current
						{:else}
							There are no differences
						{/if}
					</Alert>
				{/if}
			{:else}
				<Loader2 class="animate-spin" />
			{/if}
		</div>
		{#snippet actions()}
			{#if data?.mode === 'normal' && data.onTakeLatest}
				<Button unifiedSize="sm" variant="default" loading={takingLatest} onClick={takeLatest}>
					Take latest, keep my edits
				</Button>
			{/if}
			{#if data?.mode === 'normal'}
				<Button
					unifiedSize="sm"
					variant="default"
					onClick={restoreDeployed}
					disabled={!data.draft &&
						orderedJsonStringify(data.deployed) === orderedJsonStringify(data.current)}
				>
					Restore to deployed{data.draft ? ' and discard draft' : ''}
				</Button>
			{/if}
			{#if data?.button}
				<Button
					variant="subtle"
					onClick={() => {
						if (data?.button) {
							data.button.onClick()
							diffViewer?.closeDrawer()
						}
					}}>{data.button.text}</Button
				>
			{/if}
		{/snippet}
	</DrawerContent>
</Drawer>
