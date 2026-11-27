<script lang="ts">
	import {
		Eye,
		EyeOff,
		File,
		FileImage,
		FileSpreadsheet,
		FileText,
		PanelRightOpen
	} from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import FileDownload from './FileDownload.svelte'
	import ResultItemHeader from './ResultItemHeader.svelte'

	interface Props {
		s3object: { s3: string; storage?: string; disable_download?: boolean; presigned?: string }
		workspaceId?: string | undefined
		appPath?: string | undefined
		/** Opens the object store explorer on this object; omitted when the viewer cannot use it. */
		onExplore?: (() => void) | undefined
		/** Set when the preview is collapsible (list rows); undefined hides the toggle. */
		previewOpen?: boolean | undefined
		onTogglePreview?: (() => void) | undefined
		/** Set for a markdown file whose preview is shown; undefined hides the switch. */
		markdownSource?: boolean | undefined
		onMarkdownSource?: ((source: boolean) => void) | undefined
	}

	let {
		s3object,
		workspaceId = undefined,
		appPath = undefined,
		onExplore = undefined,
		previewOpen = undefined,
		onTogglePreview = undefined,
		markdownSource = undefined,
		onMarkdownSource = undefined
	}: Props = $props()

	let key = $derived(s3object?.s3 ?? '')
	let filename = $derived(key.split('/').pop() || key)
	let extension = $derived(filename.includes('.') ? filename.split('.').pop()?.toLowerCase() : '')
	let FileIcon = $derived(
		['png', 'jpg', 'jpeg', 'webp', 'gif', 'svg'].includes(extension ?? '')
			? FileImage
			: ['csv', 'parquet'].includes(extension ?? '')
				? FileSpreadsheet
				: ['pdf', 'txt', 'log', 'md', 'markdown', 'json'].includes(extension ?? '')
					? FileText
					: File
	)
</script>

<ResultItemHeader icon={FileIcon} title={filename} subtitleTooltip={key}>
	{#snippet subtitle()}
		S3 object{#if s3object?.storage}&nbsp;in {s3object.storage}{/if} ·
		<span class="font-mono">{key}</span>
	{/snippet}
	{#snippet actions()}
		{#if markdownSource !== undefined && onMarkdownSource}
			<ToggleButtonGroup
				noWFull
				selected={markdownSource ? 'text' : 'rendered'}
				onSelected={(v) => onMarkdownSource(v === 'text')}
				class="shrink-0 mr-1"
			>
				{#snippet children({ item })}
					<ToggleButton value="rendered" icon={Eye} iconOnly tooltip="Rendered" size="sm" {item} />
					<ToggleButton value="text" icon={FileText} iconOnly tooltip="Text" size="sm" {item} />
				{/snippet}
			</ToggleButtonGroup>
		{/if}
		{#if onTogglePreview && previewOpen !== undefined}
			<Button
				variant="subtle"
				unifiedSize="sm"
				startIcon={{ icon: previewOpen ? EyeOff : Eye }}
				onClick={onTogglePreview}
			>
				{previewOpen ? 'Hide preview' : 'Preview'}
			</Button>
		{/if}
		<!-- The file browser has its own Download button, so it would undo disable_download. -->
		{#if onExplore && !s3object?.disable_download}
			<Tooltip>
				<Button
					variant="subtle"
					unifiedSize="sm"
					startIcon={{ icon: PanelRightOpen }}
					onClick={onExplore}
				>
					Explore
				</Button>
				{#snippet text()}
					Open in the S3 file browser. Requires admin privilege or "S3 resource details and content
					can be accessed by all users of this workspace" in the workspace's S3 storage settings.
				{/snippet}
			</Tooltip>
		{/if}
		{#if s3object?.disable_download}
			<span class="text-2xs text-hint px-2">Download disabled</span>
		{:else}
			<FileDownload {workspaceId} {s3object} {appPath} />
		{/if}
	{/snippet}
</ResultItemHeader>
