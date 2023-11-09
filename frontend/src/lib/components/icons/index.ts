import AirtableIcon from './AirtableIcon.svelte'
import DbIcon from './DbIcon.svelte'
import DiscordIcon from './DiscordIcon.svelte'
import GcalIcon from './GcalIcon.svelte'
import GCloudIcon from './GCloudIcon.svelte'
import GdriveIcon from './GdriveIcon.svelte'
import GithubIcon from './GithubIcon.svelte'
import GitlabIcon from './GitlabIcon.svelte'
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
import SendflakeIcon from './SendflakeIcon.svelte'
import QRCodeIcon from './QRCodeIcon.svelte'
import LinkedinIcon from './LinkedinIcon.svelte'
import HubspotIcon from './HubspotIcon.svelte'
import DatadogIcon from './DatadogIcon.svelte'
import StripeIcon from './StripeIcon.svelte'
import TelegramIcon from './TelegramIcon.svelte'
import FunkwhaleIcon from './FunkwhaleIcon.svelte'
import GdocsIcon from './GdocsIcon.svelte'
import NextcloudIcon from './NextcloudIcon.svelte'
import FaunadbIcon from './FaunadbIcon.svelte'
import ClickhouseIcon from './ClickhouseIcon.svelte'
import OpenaiIcon from './OpenaiIcon.svelte'
import AppwriteIcon from './AppwriteIcon.svelte'
import LinkdingIconSvelte from './LinkdingIcon.svelte'
import AwsIcon from './AwsIcon.svelte'
import BcryptIcon from './BcryptIcon.svelte'
import GoogleIcon from './GoogleIcon.svelte'
import MicrosoftIcon from './MicrosoftIcon.svelte'
import HackernewsIcon from './HackernewsIcon.svelte'
import MongodbIcon from './MongodbIcon.svelte'
import RedditIcon from './RedditIcon.svelte'
import SupabaseIcon from './SupabaseIcon.svelte'
import WebdavIcon from './WebdavIcon.svelte'
import ZammadIcon from './ZammadIcon.svelte'
import SurrealdbIcon from './SurrealdbIcon.svelte'
import SquareIcon from './SquareIcon.svelte'
import ActivitypubIcon from './ActivitypubIcon.svelte'
import AwsEcrIcon from './AwsEcrIcon.svelte'
import CalcomIcon from './CalcomIcon.svelte'
import ClickupIcon from './ClickupIcon.svelte'
import CloudflareIcon from './CloudflareIcon.svelte'
import FirebaseIcon from './FirebaseIcon.svelte'
import GoogleFormsIcon from './GoogleFormsIcon.svelte'
import JiraIcon from './JiraIcon.svelte'
import NotionIcon from './NotionIcon.svelte'
import PineconeIcon from './PineconeIcon.svelte'
import RssIcon from './RssIcon.svelte'
import ShopifyIcon from './ShopifyIcon.svelte'
import TypeformIcon from './TypeformIcon.svelte'
import BigQueryIcon from './BigQueryIcon.svelte'
import GraphqlIcon from './GraphqlIcon.svelte'
import NocoDbIcon from './NocoDbIcon.svelte'
import AzureIcon from './AzureIcon.svelte'
import OktaIcon from './OktaIcon.svelte'
import MsSqlServerIcon from './MSSqlServerIcon.svelte'

export const APP_TO_ICON_COMPONENT = {
	postgresql: PostgresIcon,
	mysql: Mysql,
	smtp: Mail,
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
	snowflake: SendflakeIcon,
	ms_sql_server: MsSqlServerIcon,
	qrcode: QRCodeIcon,
	linkedin: LinkedinIcon,
	hubspot: HubspotIcon,
	datadog: DatadogIcon,
	stripe: StripeIcon,
	telegram: TelegramIcon,
	funkwhale: FunkwhaleIcon,
	helper: WindmillIcon,
	windmillhub: WindmillIcon,
	gdocs: GdocsIcon,
	ocs: NextcloudIcon,
	faunadb: FaunadbIcon,
	clickhouse: ClickhouseIcon,
	openai: OpenaiIcon,
	appwrite: AppwriteIcon,
	linkding: LinkdingIconSvelte,
	aws: AwsIcon,
	microsoft: MicrosoftIcon,
	bcrypt: BcryptIcon,
	google: GoogleIcon,
	hackernews: HackernewsIcon,
	mongodb: MongodbIcon,
	reddit: RedditIcon,
	supabase: SupabaseIcon,
	square: SquareIcon,
	webdav: WebdavIcon,
	zammad: ZammadIcon,
	nextcloud: NextcloudIcon,
	surrealdb: SurrealdbIcon,
	activitypub: ActivitypubIcon,
	aws_ecr: AwsEcrIcon,
	calcom: CalcomIcon,
	clickup: ClickupIcon,
	cloudflare: CloudflareIcon,
	firebase: FirebaseIcon,
	gforms: GoogleFormsIcon,
	jira: JiraIcon,
	notion: NotionIcon,
	pinecone: PineconeIcon,
	rss: RssIcon,
	shopify: ShopifyIcon,
	typeform: TypeformIcon,
	bigquery: BigQueryIcon,
	graphql: GraphqlIcon,
	nocodb: NocoDbIcon,
	azure: AzureIcon,
	okta: OktaIcon
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
	LinkedinIcon,
	HubspotIcon,
	TelegramIcon,
	StripeIcon,
	DatadogIcon,
	FunkwhaleIcon,
	GdocsIcon,
	FaunadbIcon,
	ClickhouseIcon,
	OpenaiIcon,
	AwsIcon,
	BcryptIcon,
	GoogleIcon,
	HackernewsIcon,
	MongodbIcon,
	RedditIcon,
	SupabaseIcon,
	WebdavIcon,
	ZammadIcon,
	NextcloudIcon,
	SendflakeIcon,
	SurrealdbIcon,
	ActivitypubIcon,
	AwsEcrIcon,
	CalcomIcon,
	ClickupIcon,
	CloudflareIcon,
	FirebaseIcon,
	GoogleFormsIcon,
	JiraIcon,
	NotionIcon,
	PineconeIcon,
	RssIcon,
	ShopifyIcon,
	TypeformIcon,
	BigQueryIcon,
	GraphqlIcon,
	NocoDbIcon,
	AzureIcon,
	MicrosoftIcon,
	OktaIcon
}
