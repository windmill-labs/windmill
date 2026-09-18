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

	export function openDrawer(token?: number) {
		// No token means the caller is taking the drawer for itself, so claim one here:
		// every reuse (a deploy-override's "Show diff", a draft badge, a workspace
		// comparison) then invalidates an editor opening that is still fetching, instead
		// of being replaced by it when it lands.
		if (token != null && token !== openingToken) return
		if (token == null) openingToken++
		data = undefined
		diffType = undefined
		diffViewer?.openDrawer()
	}

	export function closeDrawer() {
		diffViewer?.closeDrawer()
	}

	/** Counted per opening, and counted here rather than in the editor that opens one: a
	 *  path change remounts the editor while this drawer stays, so a counter local to it is
	 *  one an outlived request still matches, and that request would fill the drawer with
	 *  the item the user just left. Every write an opening makes checks this first. */
	let openingToken = 0

	export function beginOpening(): number {
		return ++openingToken
	}

	export function ownsOpening(token: number): boolean {
		return token === openingToken
	}

	/** Drop an opening and everything it put on screen: its editor is going away, so a diff
	 *  it filled acts on an item that is gone (Take latest would move the base of whatever
	 *  loaded in its place) and one still in flight would leave the spinner behind. A no-op
	 *  once another opening owns the drawer: what it shows is then someone else's. */
	export function abandonOpening(token: number) {
		if (token !== openingToken) return
		openingToken++
		// The version load and the page in flight, if any, belong to the diff being dropped.
		versionLoadGeneration++
		versionListGeneration++
		loadingVersion = false
		data = undefined
		diffType = undefined
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
	/** Counted separately from `versionLoadGeneration`: only a diff replacing this one
	 *  invalidates a page in flight. Picking a version while one loads is not a reason to
	 *  drop it — the editor's cursor has already moved past that page, so discarding it
	 *  would skip it until the drawer is reopened. */
	let versionListGeneration = 0
	/** Pages of `versions` fetched after the first. Kept beside `data` so a diff swapped in
	 *  by a newer opening drops them along with the list they extended. */
	let extraVersions: DiffVersionOption[] = $state([])
	let moreLoader: (() => Promise<DiffVersionOption[] | undefined>) | undefined = $state(undefined)
	let loadingMore = $state(false)
	/** What the picker offers: the page the editor handed over plus whatever the reader has
	 *  asked for since. Every read of the version list goes through this. */
	const pickerVersions = $derived.by(() =>
		data?.mode === 'normal' ? [...(data.versions ?? []), ...extraVersions] : []
	)

	/** The editors hand over one page, so a path a pipeline has deployed thousands of times
	 *  does not hold the drawer shut while its whole history arrives. */
	async function fetchMoreVersions() {
		if (!moreLoader || loadingMore) return
		loadingMore = true
		const generation = versionListGeneration
		try {
			const more = await moreLoader()
			// A diff swapped in while this ran owns the picker now; appending would splice
			// one item's history onto another's.
			if (generation !== versionListGeneration) return
			if (more?.length) {
				extraVersions = [...extraVersions, ...more]
			} else {
				// Nothing came back, so there is nothing further to ask for.
				moreLoader = undefined
			}
		} catch (e: any) {
			if (generation === versionListGeneration) {
				sendUserToast(`Could not load older versions: ${e?.body ?? e?.message ?? e}`, true)
			}
		} finally {
			loadingMore = false
		}
	}

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
			const opt = pickerVersions.find((v) => v.id === id)
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
			? (pickerVersions.find((v) => v.isHead)?.id ?? data.deployedHead)
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
		// Persisting the base is awaited, and this drawer outlives the editor that filled
		// it: close only the opening this action belongs to, or it takes down whichever
		// diff was opened meanwhile.
		const opening = openingToken
		try {
			await data.onTakeLatest(headShown)
			if (opening === openingToken) diffViewer?.closeDrawer()
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
					loadMoreVersions?: () => Promise<DiffVersionOption[] | undefined>
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
			  },
		token?: number
	) {
		// Same rule as `openDrawer`: an opening's own token has to still be current, and a
		// caller with none is taking the drawer, so it claims one.
		if (token != null && token !== openingToken) return
		if (token == null) openingToken++
		if (diff.mode === 'normal') {
			const {
				deployed,
				deployedLabel,
				versions,
				loadVersion,
				loadMoreVersions,
				onTakeLatest,
				draftBase,
				deployedHead,
				draft,
				current,
				button
			} = diff
			versionLoader = loadVersion
			moreLoader = loadMoreVersions
			headLabel = deployedLabel
			headDeployed = !deployed.draft_only ? prepareDiff(deployed) : undefined
			// A load or page still in flight belongs to the diff being replaced.
			versionLoadGeneration++
			versionListGeneration++
			loadingVersion = false
			extraVersions = []
			loadingMore = false
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
				{#if data.mode === 'normal' && (data.deployedLabel || pickerVersions.length)}
					<div class="flex gap-2 items-center text-xs text-secondary">
						{#if pickerVersions.length && versionLoader}
							<div class="w-72">
								<Select
									items={pickerVersions.map((v) => ({
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
							{#if moreLoader}
								<Button
									variant="subtle"
									unifiedSize="2xs"
									disabled={loadingMore}
									on:click={fetchMoreVersions}>Load older</Button
								>
							{/if}
							{#if loadingMore}
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
