<script lang="ts">
	import { goto } from '$app/navigation'

	import { sendUserToast } from '$lib/utils'
	import { faSpinner } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'

	sendUserToast('Sucess. You can return to your terminal now.')
	goto('/')
</script>

<Icon data={faSpinner} class={`animate-spin`} />
