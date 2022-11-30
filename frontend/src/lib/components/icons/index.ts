import AirtableIcon from './AirtableIcon.svelte'
import DbIcon from './DbIcon.svelte'
import DiscordIcon from './DiscordIcon.svelte'
import GcalIcon from './GcalIcon.svelte'
import GCloudIcon from './GCloudIcon.svelte'
import GdriveIcon from './GdriveIcon.svelte'
import GithubIcon from './GithubIcon.svelte'
import GitlabIcon from './GItlabIcon.svelte'
import GmailIcon from './GmailIcon.svelte'
import GSheetsIcon from './GSheetsIcon.svelte'
import HttpIcon from './HttpIcon.svelte'
import Mail from './Mail.svelte'
import MastodonIcon from './MastodonIcon.svelte'
import MatrixIcon from './MatrixIcon.svelte'
import Mysql from './Mysql.svelte'
import PostgresIcon from './PostgresIcon.svelte'
import S3Icon from './S3Icon.svelte'
import Slack from './Slack.svelte'
import TogglIcon from './TogglIcon.svelte'
import WindmillIcon from './WindmillIcon.svelte'
import MailchimpIcon from './MailchimpIcon.svelte'
import SendgridIcon from './SendgridIcon.svelte'

export const APP_TO_ICON_COMPONENT =  {
	postgresql: PostgresIcon,
	mysql: Mysql,
	smtp: Mail,
	mongodb: DbIcon,
	slack: Slack,
	github: GithubIcon,
	gmail: GmailIcon,
	gsheets: GSheetsIcon,
	gitlab: GitlabIcon,
	gcloud: GCloudIcon,
	gcal: GcalIcon,
	gdrive: GdriveIcon,
	airtable: AirtableIcon,
	toggl: TogglIcon,
	s3: S3Icon,
	discord: DiscordIcon,
	discord_webhook: DiscordIcon,
	mastodon: MastodonIcon,
	matrix: MatrixIcon,
	windmill: WindmillIcon,
	http: HttpIcon,
	mailchimp: MailchimpIcon,
	sendgrid: SendgridIcon,
} as const

export {
	AirtableIcon,
	DbIcon,
	DiscordIcon,
	GcalIcon,
	GCloudIcon,
	GdriveIcon,
	GithubIcon,
	GitlabIcon,
	GmailIcon,
	GSheetsIcon,
	HttpIcon,
	Mail,
	MastodonIcon,
	MatrixIcon,
	Mysql,
	PostgresIcon,
	S3Icon,
	Slack,
	TogglIcon,
	WindmillIcon,
	MailchimpIcon,
	SendgridIcon,
}