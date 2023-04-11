<script context="module">
	export function load({ params }) {
		return {
			stuff: { title: `Script ${params.path}` }
		}
	}
</script>

<script lang="ts">
	import { page } from '$app/stores'
	import { workspaceStore } from '$lib/stores'
	import { goto } from '$app/navigation'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { WindmillIcon } from '$lib/components/icons'
	import { ScriptService } from '$lib/gen'

	async function redirectMe() {
		if ($workspaceStore) {
			const script = await ScriptService.getScriptByPath({
				workspace: $workspaceStore,
				path: $page.params.path
			})
			const url = new URL($page.url.origin + '/scripts/run/' + script.hash)
			$page.url.searchParams.forEach((v, k) => url.searchParams.append(k, v))
			await goto(url)
		} else {
			await goto('/user/workspaces')
		}
	}

	redirectMe()
</script>

<CenteredModal title="Redirecting to latest Script Version...">
	<div class="w-full">
		<div class="block m-auto w-20">
			<WindmillIcon height="80px" width="80px" spin="fast" />
		</div>
	</div>
</CenteredModal>
