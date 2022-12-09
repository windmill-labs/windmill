<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { classNames } from '$lib/utils'
	import { faRefresh } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../types'

	const { runnableComponents } = getContext<AppEditorContext>('AppEditorContext')

	let loading: boolean = false
	let disabled: boolean = false
	async function onRefresh() {
		await Promise.all(
			Object.keys($runnableComponents).map((id) => {
				return $runnableComponents?.[id]?.runComponent()
			})
		)
		loading = false
	}

	$: $runnableComponents && (disabled = Object.keys($runnableComponents).length === 0)
</script>

<Button
	size="xs"
	btnClasses="m-2 mb-4"
	startIcon={{ icon: faRefresh, classes: classNames(loading ? 'animate-spin' : '', 'mr-2') }}
	color="light"
	variant="border"
	{disabled}
	on:click={() => {
		loading = true
		onRefresh()
	}}
>
	Recompute all
</Button>
