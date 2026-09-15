<script lang="ts">
	import { Check, Copy, Download } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { copyToClipboard, download } from '$lib/utils'
	import { artifactFilename, artifactMimeType, type ArtifactKind } from './artifactsDB'

	interface Props {
		name: string
		kind: ArtifactKind
		content: string
		disabled?: boolean
	}

	let { name, kind, content, disabled = false }: Props = $props()

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
	startIcon={{ icon: copied ? Check : Copy }}
	onClick={copyRaw}
	title="Copy raw markdown"
	dropdownItems={[
		{
			label: 'Download as .md',
			icon: Download,
			onClick: () => download(artifactFilename({ name, kind }), content, artifactMimeType(kind)),
			disabled
		}
	]}
>
	{copied ? 'Copied' : 'Copy'}
</Button>
