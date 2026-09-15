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
	import { sendUserToast } from '$lib/toast'
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
				onTakeLatest?: (head?: string) => void | Promise<void>
				draftBase?: string
				deployedHead?: string
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

	/** Counted per opening, and counted here rather than in the editor that opens one: a
	 *  path change remounts the editor while this drawer stays mounted, so a counter local
	 *  to the editor is one an outlived request still matches — it would open and fill the
	 *  drawer with the item the user just left. Every write an opening makes (the blanking
	 *  `openDrawer` included) checks `ownsOpening` first. */
	let openingToken = 0

	export function beginOpening(): number {
		return ++openingToken
	}

	export function ownsOpening(token: number): boolean {
		return token === openingToken
	}

	/** Drop the opening in flight: the editor that started it is going away. */
	export function abandonOpening() {
		openingToken++
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
	/** The deployed head, prepared for diffing as `data.deployed` is: `data.deployed`
	 *  follows the picker while Restore always restores the head, so its enabled state
	 *  compares against this. Must hold `prepareDiff`'s output, not the raw value, or the
	 *  comparison never matches and Restore is always enabled. */
	let headDeployed: ReturnType<typeof prepareDiff> | undefined = $state(undefined)
	let loadingVersion = $state(false)
	/** Which version load the spinner belongs to, counted rather than keyed on the id so
	 *  re-picking the same version is still generation-safe. A response from any other
	 *  generation is stale — a slower earlier pick, or one outlived by a drawer reset —
	 *  and neither replaces the diff nor clears the spinner, which `disabled` rides on. */
	let versionLoadGeneration = 0

	async function selectVersion(id: string | undefined) {
		if (!id || !versionLoader || !data || data.mode !== 'normal') return
		const shown = selectedVersion
		selectedVersion = id
		const generation = ++versionLoadGeneration
		loadingVersion = true
		try {
			const value = await versionLoader(id)
			if (generation !== versionLoadGeneration) return
			if (!value || !data || data.mode !== 'normal') {
				// Nothing loaded: the diff still shows the previous version, so the picker
				// has to say so rather than name one the reader is not looking at.
				selectedVersion = shown
				if (!value) sendUserToast(`Could not load version ${id}`, true)
				return
			}
			const opt = data.versions?.find((v) => v.id === id)
			data = {
				...data,
				deployed: prepareDiff(value),
				deployedLabel: opt?.isHead ? headLabel : opt?.label
			}
		} catch (e: any) {
			// The snap-back would otherwise be the only sign the version never loaded.
			if (generation === versionLoadGeneration) {
				selectedVersion = shown
				sendUserToast(`Could not load version ${id}: ${e?.body ?? e?.message ?? e}`, true)
			}
		} finally {
			if (generation === versionLoadGeneration) loadingVersion = false
		}
	}

	/** The version this drawer presents as the deployed head: the one its list marks, else
	 *  the head the editor knows. Both the action's gate and the base it adopts hang off
	 *  it, so "take latest" means the version the reader is looking at. */
	const headShown = $derived.by(() =>
		data?.mode === 'normal'
			? (data.versions?.find((v) => v.isHead)?.id ?? data.deployedHead)
			: undefined
	)
	/** Behind as the drawer can see it. Unknown counts as not behind: offering to adopt a
	 *  head nobody could name would move the base to a version never shown. */
	const behindShown = $derived.by(
		() =>
			data?.mode === 'normal' &&
			data.draftBase != null &&
			headShown != null &&
			data.draftBase !== headShown
	)

	let takingLatest = $state(false)
	async function takeLatest() {
		if (!data || data.mode !== 'normal' || !data.onTakeLatest || takingLatest) return
		takingLatest = true
		try {
			await data.onTakeLatest(headShown)
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
					onTakeLatest?: (head?: string) => void | Promise<void>
					draftBase?: string
					deployedHead?: string
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
				draftBase,
				deployedHead,
				draft,
				current,
				button
			} = diff
			versionLoader = loadVersion
			headLabel = deployedLabel
			headDeployed = !deployed.draft_only ? prepareDiff(deployed) : undefined
			// A load still in flight belongs to the diff being replaced.
			versionLoadGeneration++
			loadingVersion = false
			selectedVersion = versions?.find((v) => v.isHead)?.id
			data = {
				mode: 'normal',
				deployed: !deployed.draft_only ? prepareDiff(deployed) : undefined,
				deployedLabel,
				versions,
				onTakeLatest,
				draftBase,
				deployedHead,
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
			{#if data?.mode === 'normal' && data.onTakeLatest && behindShown}
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
						orderedJsonStringify(headDeployed ?? data.deployed) ===
							orderedJsonStringify(data.current)}
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
