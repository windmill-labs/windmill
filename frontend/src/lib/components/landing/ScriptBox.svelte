<script lang="ts">
	import { goto } from '$app/navigation'
	import type { Script } from '$lib/gen'

	import { truncateHash } from '$lib/utils'
	import { faPencil, faPlay } from '@fortawesome/free-solid-svg-icons'
	import { Button, Badge } from '$lib/components/common'
	import { LanguageIcon } from '../common/languageIcons'

	export let script: Script
</script>

<a
	class="border border-gray-400 p-4 rounded-sm shadow-sm space-y-2 hover:border-blue-600 text-gray-800 flex flex-col justify-between"
	href="/scripts/get/{script.hash}"
>
	<div class="font-bold">{script.summary || script.path}</div>

	<div class="inline-flex justify-between w-full">
		<div class="text-xs">{script.path}</div>
		<div><LanguageIcon height={16} lang={script.language} /></div>
	</div>
	<div class="inline-flex space-x-1 w-full">
		{#if script.kind !== 'script'}
			<Badge color="green" capitalize>
				{script.kind}
			</Badge>
		{/if}
	</div>
	<div class="flex flex-row-reverse gap-x-2">
		<Button
			href="/scripts/edit/{script.hash}?step=2"
			color="dark"
			size="xs"
			variant="border"
			startIcon={{ icon: faPencil }}
		>
			Edit
		</Button>

		<Button
			href="/scripts/run/{script.hash}"
			color="dark"
			size="xs"
			variant="border"
			startIcon={{ icon: faPlay }}
		>
			Run
		</Button>
	</div>
</a>
