<script lang="ts">
	import { type UserExt } from '$lib/stores'
	import RawAppBackgroundRunner from './RawAppBackgroundRunner.svelte'
	import type { Runnable } from './rawAppPolicy'
	import { htmlContent } from './utils'
	import { onMount } from 'svelte'

	interface Props {
		workspace: string
		user: UserExt | undefined
		secret: string | undefined
		path: string
		runnables: Record<string, Runnable>
	}

	let { workspace, user, secret, path, runnables }: Props = $props()

	let iframe = $state() as HTMLIFrameElement | undefined

	// Get initial hash from parent URL to pass to iframe
	let initialHash = $state('')

	onMount(() => {
		initialHash = window.location.hash || ''
	})

	// Use blob URL instead of srcDoc to give the iframe a proper origin.
	// srcDoc iframes have "null" origin which breaks URL constructor in routers.
	let blobUrl = $derived.by(() => {
		if (!secret) return undefined
		const baseUrl = typeof window !== 'undefined' ? window.location.origin : ''
		const html = htmlContent(workspace, secret, { ctx: user, workspace }, baseUrl, initialHash)
		const blob = new Blob([html], { type: 'text/html' })
		return URL.createObjectURL(blob)
	})

	// Cleanup blob URL when it changes or component unmounts
	$effect(() => {
		const url = blobUrl
		return () => {
			if (url) URL.revokeObjectURL(url)
		}
	})

	// Listen for hash changes from iframe and update parent URL
	$effect(() => {
		function handleMessage(event: MessageEvent) {
			console.log('[Parent] Received message:', event.data)
			if (event.data?.type === 'windmill:hashchange') {
				const newHash = event.data.hash || ''
				console.log('[Parent] Updating hash to:', newHash)
				// Update parent URL without triggering navigation
				if (window.location.hash !== newHash) {
					history.replaceState(null, '', newHash || window.location.pathname)
				}
			}
		}

		window.addEventListener('message', handleMessage)
		return () => window.removeEventListener('message', handleMessage)
	})
</script>

<RawAppBackgroundRunner {workspace} editor={false} {iframe} {runnables} {path} />

{#if blobUrl}
	<iframe
		bind:this={iframe}
		title="raw-app"
		src={blobUrl}
		class="w-full h-full min-h-screen bg-white border-none"
	></iframe>
{/if}
