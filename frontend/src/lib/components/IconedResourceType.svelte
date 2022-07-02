<script lang="ts">
	import Mysql from './icons/Mysql.svelte'
	import Mail from './icons/Mail.svelte'
	import DbIcon from './icons/DbIcon.svelte'
	import PostgresIcon from './icons/PostgresIcon.svelte'
	import { faGithub } from '@fortawesome/free-brands-svg-icons'
	import Slack from './icons/Slack.svelte'
	import Icon from 'svelte-awesome'

	export let name: string
	export let after: boolean = false
	export let height = '24px'
	export let width = '24px'
</script>

<div class="flex flex-row gap-2">
	{#if !after}
		{name}
	{/if}
	{#if name === 'postgresql'}
		<PostgresIcon {height} {width} />
	{:else if name === 'mysql'}
		<Mysql {height} {width} />
	{:else if name === 'smtp'}
		<Mail {height} {width} />
	{:else if name === 'mongodb'}
		<DbIcon {height} {width} />
	{:else if name === 'slack'}
		<Slack {height} {width} />
	{:else if name === 'github'}
		<Icon data={faGithub} scale={1.4} />
	{/if}
	{#if after}
		{name}
	{/if}
</div>
