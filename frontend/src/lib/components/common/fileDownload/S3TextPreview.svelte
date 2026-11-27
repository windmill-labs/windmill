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

	function request(
		key: string,
		storage: string | undefined,
		presigned: string | undefined,
		length: number
	) {
		const workspace = workspaceId ?? $operatingWorkspace!
		const range = { readBytesFrom: 0, readBytesLength: length }
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

	// One byte past the limit tells a file of exactly MAX_BYTES from a longer one. The server
	// refuses a read that ends inside a UTF-8 character (up to 4 bytes), answering with no
	// content and this message: each retry reads one byte less. `readLimit` is the length read.
	const CUT_CHARACTER = /File encoding is not supported/
	async function load(key: string, storage: string | undefined, presigned: string | undefined) {
		for (let length = MAX_BYTES + 1; ; length--) {
			const res = await request(key, storage, presigned, length)
			if (res.content || !CUT_CHARACTER.test(res.msg ?? '') || length <= MAX_BYTES - 2) {
				return { res, readLimit: length }
			}
		}
	}

	let preview = $derived(load(s3object.s3, s3object.storage, s3object.presigned))
</script>

{#await preview}
	<div class="flex items-center gap-1 px-3 py-2 text-2xs text-secondary">
		<Loader2 size={12} class="animate-spin" /> Loading preview
	</div>
{:then { res, readLimit }}
	{#if res.content_type === 'Unknown' || !res.content}
		<div class="px-3 py-2 text-2xs text-secondary">{res.msg || 'No preview available.'}</div>
	{:else}
		<FileTextPreview content={res.content} {readLimit} {markdown} {markdownSource} />
	{/if}
{:catch e}
	<div class="px-3 py-2 text-2xs text-red-500 break-all">
		Could not load the preview: {e?.body ?? e?.message ?? e}
	</div>
{/await}
