<script lang="ts" module>
	import { lazyModule } from '$lib/lazyModule.svelte'
	import type { UserDraftItemKind } from '$lib/gen'

	// It reaches the AI clients, which the transcript must not load to render a card.
	const diffDrawerModule = lazyModule(() => import('$lib/components/DiffDrawer.svelte'))

	export type DeployDiffSides = { kind: UserDraftItemKind; deployed: unknown; draft: unknown }
</script>

<script lang="ts">
	import { tick } from 'svelte'
	import { GitCompareArrows } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import type DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import { sendUserToast } from '$lib/toast'
	import type { Value } from '$lib/utils'

	interface Props {
		/** Undefined disables the button. */
		load: (() => Promise<DeployDiffSides>) | undefined
	}

	let { load }: Props = $props()

	let diffDrawer: DiffDrawer | undefined = $state(undefined)
	let isFlow = $state(false)
	let loading = $state(false)

	async function showDiff() {
		if (!load || loading) return
		loading = true
		try {
			const [diff] = await Promise.all([load(), diffDrawerModule.load()])
			isFlow = diff.kind === 'flow'
			await tick()
			diffDrawer?.openDrawer()
			diffDrawer?.setDiff({
				mode: 'simple',
				title: 'Deployed → Draft',
				original: diff.deployed as Value,
				current: diff.draft as Value
			})
		} catch (e: any) {
			sendUserToast(`Could not load the diff: ${e?.body ?? e?.message ?? e}`, true)
		} finally {
			loading = false
		}
	}
</script>

<Button
	variant="subtle"
	unifiedSize="sm"
	startIcon={{ icon: GitCompareArrows }}
	{loading}
	disabled={!load}
	title="Compare the draft with what is deployed"
	onClick={showDiff}
>
	Diff
</Button>

{#if diffDrawerModule.current}
	{@const LoadedDiffDrawer = diffDrawerModule.current.default}
	<LoadedDiffDrawer bind:this={diffDrawer} {isFlow} />
{/if}
