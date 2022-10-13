<script lang="ts">
	import Icon from 'svelte-awesome'
	import AirtableIcon from './icons/AirtableIcon.svelte'
	import DbIcon from './icons/DbIcon.svelte'
	import DiscordIcon from './icons/DiscordIcon.svelte'
	import GcalIcon from './icons/GcalIcon.svelte'
	import GCloudIcon from './icons/GCloudIcon.svelte'
	import GdriveIcon from './icons/GdriveIcon.svelte'
	import GithubIcon from './icons/GithubIcon.svelte'
	import GitlabIcon from './icons/GItlabIcon.svelte'
	import GmailIcon from './icons/GmailIcon.svelte'
	import GSheetsIcon from './icons/GSheetsIcon.svelte'
	import HttpIcon from './icons/HttpIcon.svelte'
	import Mail from './icons/Mail.svelte'
	import MastodonIcon from './icons/MastodonIcon.svelte'
	import MatrixIcon from './icons/MatrixIcon.svelte'
	import Mysql from './icons/Mysql.svelte'
	import PostgresIcon from './icons/PostgresIcon.svelte'
	import S3Icon from './icons/S3Icon.svelte'
	import Slack from './icons/Slack.svelte'
	import TogglIcon from './icons/TogglIcon.svelte'
	import WindmillIcon from './icons/WindmillIcon.svelte'

	export let name: string
	export let silent = false
	export let after: boolean = false
	export let height = '24px'
	export let width = '24px'
</script>

<div class="flex flex-row gap-2 justify-center items-center -pl-2 whitespace-nowrap">
	{#if !silent && !after}
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
		<GithubIcon />
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
	{:else if name === 's3'}
		<S3Icon {height} {width} />
	{:else if name === 'discord' || name == 'discord_webhook'}
		<DiscordIcon {height} {width} />
	{:else if name === 'mastodon'}
		<MastodonIcon {height} {width} />
	{:else if name === 'matrix'}
		<MatrixIcon {height} {width} />
	{:else if name === 'windmill'}
		<WindmillIcon {height} {width} />
	{:else if name === 'http'}
		<HttpIcon {height} {width} />
	{/if}
	{#if !silent && after}
		{name}
	{/if}
</div>
