<script lang="ts">
	import { Check, Copy, Download, FileCode, FileDown } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { copyToClipboard, download } from '$lib/utils'
	import { artifactFilename, artifactMimeType, type ArtifactKind } from './artifactsDB'
	import { artifactHtmlFile } from './artifactExport'
	import { printArtifact } from './printArtifact'

	interface Props {
		name: string
		kind: ArtifactKind
		content: string
		disabled?: boolean
		/** The icon alone, for a bar too narrow for the label. */
		iconOnly?: boolean
	}

	let { name, kind, content, disabled = false, iconOnly = false }: Props = $props()

	let copied = $state(false)
	async function copyRaw() {
		if (!(await copyToClipboard(content))) return
		copied = true
		setTimeout(() => (copied = false), 1500)
	}
</script>

<!-- A Button's `disabled` does not reach its dropdown items, so the item carries its own. -->
<Button
	unifiedSize="sm"
	variant="default"
	{disabled}
	{iconOnly}
	startIcon={{ icon: copied ? Check : Copy }}
	onClick={copyRaw}
	title="Copy raw markdown"
	dropdownItems={[
		{
			label: `Download as .${kind}`,
			icon: Download,
			onClick: () => download(artifactFilename({ name, kind }), content, artifactMimeType(kind)),
			disabled
		},
		// Markdown only: an html artifact already downloads as .html, and printing it would run its
		// scripts in this origin.
		...(kind === 'md'
			? [
					{
						label: 'Download as .html',
						icon: FileCode,
						onClick: async () =>
							download(
								artifactFilename({ name, kind: 'html' }),
								await artifactHtmlFile(name, content),
								artifactMimeType('html')
							),
						disabled
					},
					{
						label: 'Download as PDF',
						icon: FileDown,
						onClick: () => printArtifact(name, content),
						disabled
					}
				]
			: [])
	]}
>
	{copied ? 'Copied' : 'Copy'}
</Button>
