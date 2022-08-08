<script lang="ts">
	import { faGithub } from '@fortawesome/free-brands-svg-icons'
	import Icon from 'svelte-awesome'
	import AirtableIcon from './icons/AirtableIcon.svelte'
	import DbIcon from './icons/DbIcon.svelte'
	import GcalIcon from './icons/GcalIcon.svelte'
	import GCloudIcon from './icons/GCloudIcon.svelte'
	import GdriveIcon from './icons/GdriveIcon.svelte'
	import GitlabIcon from './icons/GItlabIcon.svelte'
	import GmailIcon from './icons/GmailIcon.svelte'
	import GSheetsIcon from './icons/GSheetsIcon.svelte'
	import Mail from './icons/Mail.svelte'
	import Mysql from './icons/Mysql.svelte'
	import PostgresIcon from './icons/PostgresIcon.svelte'
	import Slack from './icons/Slack.svelte'
	import TogglIcon from './icons/TogglIcon.svelte'

	export let name: string
	export let after: boolean = false
	export let height = '24px'
	export let width = '24px'
</script>

<div class="flex flex-row gap-2 justify-center -pl-2 whitespace-nowrap">
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
	{:else if name === 'gmail'}
		<GmailIcon {height} {width} />
	{:else if name === 'gsheets'}
		<GSheetsIcon {height} {width} />
	{:else if name === 'gitlab'}
		<GitlabIcon {height} {width} />
	{:else if name === 'gcloud'}
		<GCloudIcon {height} {width} />
	{:else if name === 'gcal'}
		<GcalIcon {height} {width} />
	{:else if name === 'gdrive'}
		<GdriveIcon {height} {width} />
	{:else if name === 'airtable'}
		<AirtableIcon {height} {width} />
	{:else if name === 'toggl'}
		<TogglIcon {height} {width} />
	{/if}
	{#if after}
		{name}
	{/if}
</div>
