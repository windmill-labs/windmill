<script lang="ts">
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import YAML from 'yaml'
	import { Button } from '$lib/components/common'
	import { sendUserToast } from '$lib/toast'
	import { Loader2 } from 'lucide-svelte'
	import type SimpleEditor from '../SimpleEditor.svelte'
	import type { Runnable } from './utils'
	import type { RawAppData } from './dataTableRefUtils'

	export type RawAppYamlUpdate = {
		summary?: string
		files?: Record<string, string>
		runnables?: Record<string, Runnable>
		data?: RawAppData
	}

	interface Props {
		drawer?: Drawer | undefined
		summary: string
		files: Record<string, string> | undefined
		runnables: Record<string, Runnable>
		data: RawAppData
		onApply: (update: RawAppYamlUpdate) => void
	}

	let {
		drawer = $bindable(),
		summary,
		files,
		runnables,
		data,
		onApply
	}: Props = $props()

	let code = $state('')
	let initialCode = $state('')
	let editor = $state(undefined) as SimpleEditor | undefined
	let hasChanges = $derived(code !== initialCode)

	function reload() {
		const snapshot = {
			summary,
			files: files ?? {},
			runnables,
			data
		}
		code = YAML.stringify(snapshot)
		initialCode = code
		editor?.setCode(code)
	}

	function isPlainObject(value: unknown): value is Record<string, unknown> {
		return typeof value === 'object' && value !== null && !Array.isArray(value)
	}

	function apply() {
		try {
			const parsed = YAML.parse(code)
			if (!isPlainObject(parsed)) {
				throw new Error('Top-level YAML must be a mapping')
			}
			const update: RawAppYamlUpdate = {}
			if (typeof parsed.summary === 'string') {
				update.summary = parsed.summary
			}
			if (isPlainObject(parsed.files)) {
				update.files = parsed.files as Record<string, string>
			}
			if (isPlainObject(parsed.runnables)) {
				update.runnables = parsed.runnables as Record<string, Runnable>
			}
			if (isPlainObject(parsed.data)) {
				update.data = parsed.data as unknown as RawAppData
			}
			onApply(update)
			initialCode = code
			sendUserToast('Changes applied')
		} catch (e) {
			sendUserToast('Error parsing yaml: ' + e, true)
		}
	}

	let editorHeight = $state(0)
</script>

<Drawer on:open={reload} bind:this={drawer} size="800px">
	<DrawerContent title="Raw app YAML" on:close={() => drawer?.toggleDrawer()}>
		{#snippet actions()}
			<Button variant="default" unifiedSize="md" disabled={!hasChanges} on:click={reload}
				>Reset code</Button
			>
			<Button variant="accent" unifiedSize="md" disabled={!hasChanges} on:click={apply}
				>Apply changes</Button
			>
		{/snippet}

		{#await import('../SimpleEditor.svelte')}
			<Loader2 class="animate-spin" />
		{:then Module}
			<div class="h-full w-full overflow-hidden" bind:clientHeight={editorHeight}>
				<Module.default
					bind:this={editor}
					autoHeight
					minHeight={editorHeight}
					bind:code
					lang="yaml"
				/>
			</div>
		{/await}
	</DrawerContent>
</Drawer>
