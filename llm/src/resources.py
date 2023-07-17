RESOURCE_TYPES = {
    "typescript": """type Airtable = {
  apiKey: string
}

type AirtableTable = {
  baseId: string,
  tableName: string
}

type Appwrite = {
  key: string,
  project: string,
  endpoint: string,
  self_signed: boolean
}

type Aws = {
  region: string,
  awsAccessKeyId: string,
  awsSecretAccessKey: string
}

type Azure = {
  azureClientId: string,
  azureTenantId: string,
  azureClientSecret: string
}

type Clickhouse = {
  host: string,
  password: string,
  username: string
}

type CMyResourceType = {
  name: string
}

type CSolarFlar = {
  a: number,
  b: number,
  c: boolean,
  d: string,
  e: any,
  f: any[],
  g: any,
  h: any
}

type Datadog = {
  apiKey: string,
  appKey: string,
  apiBase: string
}

type DiscordBotConfiguration = {
  public_key: string,
  application_id: string
}

type DiscordWebhook = {
  webhook_url: string
}

type Dynatrace = {
  accessToken: string,
  environmentId: string,
  environmentUrl: string
}

type Faunadb = {
  region: string,
  secret: string
}

type Firebase = {
  appId: string,
  apiKey: string,
  projectId: string,
  authDomain: string,
  measurementId: string,
  storageBucket: string,
  messagingSenderId: string
}

type Funkwhale = {
  token: string,
  baseUrl: string
}

type Gcal = {
  token: string
}

type GcpServiceAccount = {
  type: string,
  auth_uri: string,
  client_id: string,
  token_uri: string,
  project_id: string,
  private_key: string,
  client_email: string,
  private_key_id: string,
  client_x509_cert_url: string,
  auth_provider_x509_cert_url: string
}

type Gdrive = {
  token: string
}

type Github = {
  token: string
}

type Gitlab = {
  token: string,
  baseUrl: string
}

type Gmail = {
  token: string
}

type Gsheets = {
  token: string
}

type Hubspot = {
  token: string
}

type Ipinfo = {
  token: string
}

type Linkding = {
  token: string,
  baseUrl: string
}

type Linkedin = {
  token: string
}

type Mailchimp = {
  server: string,
  api_key: string
}

type Mailgun = {
  api_key: string
}

type Mastodon = {
  token: string,
  baseUrl: string
}

type Matrix = {
  token: string,
  baseUrl: string
}

type Mongodb = {
  db: string,
  tls: boolean,
  servers: any,
  credential: any
}

type MongodbRest = {
  api_key: string,
  endpoint: string
}

type Mysql = {
  host: string,
  port: number,
  user: string,
  database: string,
  password: string
}

type Nocodb = {
  table: string,
  apiUrl: string,
  xc_token: string,
  workspace: string
}

type Notion = {
  token: string
}

type Openai = {
  api_key: string,
  organization_id: string
}

type Pinecone = {
  apiKey: string,
  environment: string
}

type Postgresql = {
  host: string,
  port: number,
  user: string,
  dbname: string,
  sslmode: string,
  password: string
}

type Rss = {
  url: string
}

type S3 = {
  port: number,
  bucket: string,
  region: string,
  useSSL: boolean,
  endPoint: string,
  accessKey: string,
  pathStyle: boolean,
  secretKey: string
}

type Sendgrid = {
  token: string
}

type Slack = {
  token: string
}

type Smtp = {
  host: string,
  port: number,
  user: string,
  password: string
}

type Square = {
  token: string
}

type Stripe = {
  token: string
}

type Supabase = {
  key: string,
  url: string
}

type Surrealdb = {
  url: string,
  token: string
}

type Telegram = {
  token: string
}

type Toggl = {
  token: string
}
""",
    "python": """class airtable(TypedDict):
    apiKey: str

class airtable_table(TypedDict):
    baseId: str
    tableName: str

class appwrite(TypedDict):
    key: str
    project: str
    endpoint: str
    self_signed: bool

class aws(TypedDict):
    region: str
    awsAccessKeyId: str
    awsSecretAccessKey: str

class azure(TypedDict):
    azureClientId: str
    azureTenantId: str
    azureClientSecret: str

class clickhouse(TypedDict):
    host: str
    password: str
    username: str

class c_my_resource_type(TypedDict):
    name: str

class c_solar_flar(TypedDict):
    a: int
    b: float
    c: bool
    d: str
    e: dict
    f: list
    g: dict
    h: dict

class datadog(TypedDict):
    apiKey: str
    appKey: str
    apiBase: str

class discord_bot_configuration(TypedDict):
    public_key: str
    application_id: str

class discord_webhook(TypedDict):
    webhook_url: str

class dynatrace(TypedDict):
    accessToken: str
    environmentId: str
    environmentUrl: str

class faunadb(TypedDict):
    region: str
    secret: str

class firebase(TypedDict):
    appId: str
    apiKey: str
    projectId: str
    authDomain: str
    measurementId: str
    storageBucket: str
    messagingSenderId: str

class funkwhale(TypedDict):
    token: str
    baseUrl: str

class gcal(TypedDict):
    token: str

class gcp_service_account(TypedDict):
    type: str
    auth_uri: str
    client_id: str
    token_uri: str
    project_id: str
    private_key: str
    client_email: str
    private_key_id: str
    client_x509_cert_url: str
    auth_provider_x509_cert_url: str

class gdrive(TypedDict):
    token: str

class github(TypedDict):
    token: str

class gitlab(TypedDict):
    token: str
    baseUrl: str

class gmail(TypedDict):
    token: str

class gsheets(TypedDict):
    token: str

class hubspot(TypedDict):
    token: str

class ipinfo(TypedDict):
    token: str

class linkding(TypedDict):
    token: str
    baseUrl: str

class linkedin(TypedDict):
    token: str

class mailchimp(TypedDict):
    server: str
    api_key: str

class mailgun(TypedDict):
    api_key: str

class mastodon(TypedDict):
    token: str
    baseUrl: str

class matrix(TypedDict):
    token: str
    baseUrl: str

class mongodb(TypedDict):
    db: str
    tls: bool
    servers: dict
    credential: dict

class mongodb_rest(TypedDict):
    api_key: str
    endpoint: str

class mysql(TypedDict):
    host: str
    port: float
    user: str
    database: str
    password: str

class nocodb(TypedDict):
    table: str
    apiUrl: str
    xc_token: str
    workspace: str

class notion(TypedDict):
    token: str

class openai(TypedDict):
    api_key: str
    organization_id: str

class pinecone(TypedDict):
    apiKey: str
    environment: str

class postgresql(TypedDict):
    host: str
    port: int
    user: str
    dbname: str
    sslmode: str
    password: str

class rss(TypedDict):
    url: str

class s3(TypedDict):
    port: float
    bucket: str
    region: str
    useSSL: bool
    endPoint: str
    accessKey: str
    pathStyle: bool
    secretKey: str

class sendgrid(TypedDict):
    token: str

class slack(TypedDict):
    token: str

class smtp(TypedDict):
    host: str
    port: int
    user: str
    password: str

class square(TypedDict):
    token: str

class stripe(TypedDict):
    token: str

class supabase(TypedDict):
    key: str
    url: str

class surrealdb(TypedDict):
    url: str
    token: str

class telegram(TypedDict):
    token: str

class toggl(TypedDict):
    token: str
""",
}
