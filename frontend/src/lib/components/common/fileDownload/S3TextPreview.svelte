<script lang="ts">
	import { Loader2 } from 'lucide-svelte'
	import { AppService, HelpersService } from '$lib/gen'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	import FileTextPreview from './FileTextPreview.svelte'

	interface Props {
		s3object: { s3: string; storage?: string; presigned?: string }
		workspaceId?: string | undefined
		/** Set in a deployed app: the file is read through the app-scoped `apps_u/*` route. */
		appPath?: string | undefined
		markdown?: boolean
		/** Markdown shown as its source; the caller renders the switch. */
		markdownSource?: boolean
	}

	let {
		s3object,
		workspaceId = undefined,
		appPath = undefined,
		markdown = false,
		markdownSource = undefined
	}: Props = $props()

	const operatingWorkspace = useOperatingWorkspace()

	const MAX_BYTES = 64 * 1024

	function load(key: string, storage: string | undefined, presigned: string | undefined) {
		const workspace = workspaceId ?? $operatingWorkspace!
		const range = { readBytesFrom: 0, readBytesLength: MAX_BYTES }
		if (appPath) {
			const p = new URLSearchParams(presigned ?? '')
			return AppService.appLoadFilePreview({
				workspace,
				path: appPath,
				fileKey: key,
				storage,
				...range,
				sig: p.get('sig') ?? undefined,
				exp: p.get('exp') ?? undefined
			})
		}
		return HelpersService.loadFilePreview({ workspace, fileKey: key, storage, ...range })
	}

	let preview = $derived(load(s3object.s3, s3object.storage, s3object.presigned))
</script>

{#await preview}
	<div class="flex items-center gap-1 px-3 py-2 text-2xs text-secondary">
		<Loader2 size={12} class="animate-spin" /> Loading preview
	</div>
{:then res}
	{#if res.content_type === 'Unknown' || !res.content}
		<div class="px-3 py-2 text-2xs text-secondary">{res.msg || 'No preview available.'}</div>
	{:else}
		<FileTextPreview content={res.content} readLimit={MAX_BYTES} {markdown} {markdownSource} />
	{/if}
{:catch e}
	<div class="px-3 py-2 text-2xs text-red-500 break-all">
		Could not load the preview: {e?.body ?? e?.message ?? e}
	</div>
{/await}
