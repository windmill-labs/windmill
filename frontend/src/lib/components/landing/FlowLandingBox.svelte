<script lang="ts">
	import { goto } from '$app/navigation'
	import type { Flow } from '$lib/gen'
	import { faPencil, faPlay } from '@fortawesome/free-solid-svg-icons'
	import { Button } from '../common'

	export let flow: Flow
</script>

<a
	href="#{flow.path}"
	class="border p-4 rounded-sm shadow-sm space-y-2 hover:border-blue-600 text-gray-800 flex flex-col justify-between cursor-pointer"
	on:click={() => goto(`/flows/get/${flow.path}`)}
>
	<div class="font-bold">{flow.summary || flow.path}</div>

	<div class="inline-flex justify-between w-full break-words">
		<div class="text-xs">{flow.path}</div>
	</div>

	<div class="flex flex-row-reverse gap-x-2">
		<Button
			href="/flows/edit/{flow.path}"
			color="dark"
			size="xs"
			variant="border"
			startIcon={{ icon: faPencil }}
		>
			Edit
		</Button>

		<Button
			href="/flows/run/{flow.path}"
			color="dark"
			size="xs"
			variant="border"
			startIcon={{ icon: faPlay }}
		>
			Run
		</Button>
	</div>
</a>
