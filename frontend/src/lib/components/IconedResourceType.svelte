<script lang="ts">
	import Mysql from './icons/Mysql.svelte'
	import Mail from './icons/Mail.svelte'
	import DbIcon from './icons/DbIcon.svelte'
	import PostgresIcon from './icons/PostgresIcon.svelte'
	import Icon from 'svelte-awesome'
	import { faSlack } from '@fortawesome/free-brands-svg-icons'
	import Slack from './icons/Slack.svelte'

	export let name: string
	export let after: boolean = false
	export let height = '24px'
	export let width = '24px'
</script>

<div class="flex flex-row gap-2">
	{#if !after}
		{name}
	{/if}
	{#if name === 'postgres'}
		<PostgresIcon {height} {width} />
	{:else if name === 'mysql'}
		<Mysql {height} {width} />
	{:else if name === 'SMTP'}
		<Mail {height} {width} />
	{:else if name === 'mongodb'}
		<DbIcon {height} {width} />
	{:else if name === 'slack'}
		<Slack {height} {width} />
	{/if}
	{#if after}
		{name}
	{/if}
</div>
