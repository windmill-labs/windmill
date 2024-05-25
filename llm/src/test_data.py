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
    "php": """class Stripe {
  public string $token;
}

class Bitbucket {
  public string $password;
  public string $username;
}

class Trello {
  public string $key;
  public string $token;
}
""",
}


DB_SCHEMA = """{"app":[["id","int8",true,"nextval('app_id_seq'::regclass)"],["path","varchar",true],["policy","jsonb",true],["summary","varchar",true,"''::character varying"],["versions","_int8",true],["draft_only","bool",false],["extra_perms","jsonb",true,"'{}'::jsonb"],["workspace_id","varchar",true]],"usr":[["role","varchar",false],["email","varchar",true],["disabled","bool",true,"false"],["is_admin","bool",true,"false"],["operator","bool",true,"false"],["username","varchar",true],["created_at","timestamptz",true,"now()"],["workspace_id","varchar",true]],"flow":[["path","varchar",true],["value","jsonb",true],["schema","json",false],["summary","text",true],["archived","bool",true,"false"],["edited_at","timestamptz",true,"now()"],["edited_by","varchar",true],["draft_only","bool",false],["description","text",true],["extra_perms","jsonb",true,"'{}'::jsonb"],["workspace_id","varchar",true],["dependency_job","uuid",false]],"audit":[["id","int4",true,"nextval('audit_id_seq'::regclass)"],["resource","varchar",false],["username","varchar",true],["operation","varchar",true],["timestamp","timestamptz",true,"now()"],["parameters","jsonb",false],["action_kind","action_kind",true],["workspace_id","varchar",true]],"draft":[["typ","draft_type",true],["path","varchar",true],["value","json",true],["created_at","timestamp",true,"now()"],["workspace_id","varchar",true]],"input":[["id","uuid",true],["args","jsonb",true],["name","text",true],["is_public","bool",true,"false"],["created_at","timestamptz",true,"now()"],["created_by","varchar",true],["runnable_id","varchar",true],["workspace_id","varchar",true],["runnable_type","runnable_type",true]],"queue":[["id","uuid",true],["tag","varchar",true,"'other'::character varying"],["args","jsonb",false],["logs","text",false],["email","varchar",true,"'missing@email.xyz'::character varying"],["env_id","int4",false],["running","bool",true,"false"],["suspend","int4",true,"0"],["canceled","bool",true,"false"],["job_kind","job_kind",true,"'script'::job_kind"],["language","script_lang",false,"'python3'::script_lang"],["mem_peak","int4",false],["raw_code","text",false],["raw_flow","jsonb",false],["raw_lock","text",false],["root_job","uuid",false],["last_ping","timestamptz",true,"now()"],["leaf_jobs","jsonb",false],["created_at","timestamptz",true,"now()"],["created_by","varchar",true],["parent_job","uuid",false],["started_at","timestamptz",false],["canceled_by","varchar",false],["flow_status","jsonb",false],["same_worker","bool",false,"false"],["script_hash","int8",false],["script_path","varchar",false],["is_flow_step","bool",false,"false"],["workspace_id","varchar",true],["pre_run_error","text",false],["schedule_path","varchar",false],["scheduled_for","timestamptz",true],["suspend_until","timestamptz",false],["canceled_reason","text",false],["permissioned_as","varchar",true,"'g/all'::character varying"],["concurrent_limit","int4",false],["visible_to_owner","bool",false,"true"],["concurrency_time_window_s","int4",false]],"token":[["email","varchar",false],["label","varchar",false],["owner","varchar",false],["token","varchar",true],["scopes","_text",false],["created_at","timestamptz",true,"now()"],["expiration","timestamptz",false],["super_admin","bool",true,"false"],["last_used_at","timestamptz",true,"now()"],["workspace_id","varchar",false]],"usage":[["id","varchar",true],["usage","int4",true],["month_","int4",true],["is_workspace","bool",true]],"folder":[["name","varchar",true],["owners","_varchar",true],["extra_perms","jsonb",true,"'{}'::jsonb"],["display_name","varchar",true],["workspace_id","varchar",true]],"group_":[["name","varchar",true],["summary","text",false],["extra_perms","jsonb",true,"'{}'::jsonb"],["workspace_id","varchar",true]],"script":[["tag","varchar",false],["envs","_varchar",false],["hash","int8",true],["kind","script_kind",true,"'script'::script_kind"],["lock","text",false],["path","varchar",true],["schema","json",false],["content","text",true],["deleted","bool",true,"false"],["summary","text",true],["archived","bool",true,"false"],["language","script_lang",true,"'python3'::script_lang"],["created_at","timestamptz",true,"now()"],["created_by","varchar",true],["draft_only","bool",false],["description","text",true],["extra_perms","jsonb",true,"'{}'::jsonb"],["is_template","bool",false,"false"],["workspace_id","varchar",true],["parent_hashes","_int8",false],["lock_error_logs","text",false],["concurrent_limit","int4",false],["concurrency_time_window_s","int4",false]],"account":[["id","int4",true,"nextval('account_id_seq'::regclass)"],["owner","varchar",true],["client","varchar",true],["expires_at","timestamptz",true],["workspace_id","varchar",true],["refresh_error","text",false],["refresh_token","varchar",true]],"capture":[["path","varchar",true],["payload","jsonb",true,"'null'::jsonb"],["created_at","timestamptz",true,"now()"],["created_by","varchar",true],["workspace_id","varchar",true]],"raw_app":[["data","text",true],["path","varchar",true],["summary","varchar",true,"''::character varying"],["version","int4",true,"0"],["edited_at","timestamptz",true,"now()"],["extra_perms","jsonb",true,"'{}'::jsonb"],["workspace_id","varchar",true]],"favorite":[["usr","varchar",true],["path","varchar",true],["workspace_id","varchar",true],["favorite_kind","favorite_kind",true]],"password":[["name","varchar",false],["email","varchar",true],["company","varchar",false],["verified","bool",true,"false"],["login_type","varchar",true],["super_admin","bool",true,"false"],["password_hash","varchar",false],["first_time_user","bool",true,"false"]],"resource":[["path","varchar",true],["value","jsonb",false],["description","text",false],["extra_perms","jsonb",true,"'{}'::jsonb"],["workspace_id","varchar",true],["resource_type","varchar",true]],"schedule":[["args","jsonb",false],["path","varchar",true],["email","varchar",true,"'missing@email.xyz'::character varying"],["error","text",false],["enabled","bool",true,"true"],["is_flow","bool",true,"false"],["schedule","varchar",true],["timezone","varchar",true,"'UTC'::character varying"],["edited_at","timestamptz",true,"now()"],["edited_by","varchar",true],["on_failure","varchar",false,"NULL::character varying"],["extra_perms","jsonb",true,"'{}'::jsonb"],["script_path","varchar",true],["workspace_id","varchar",true]],"variable":[["path","varchar",true],["value","varchar",true],["account","int4",false],["is_oauth","bool",true,"false"],["is_secret","bool",true,"false"],["description","varchar",true,"''::character varying"],["extra_perms","jsonb",true,"'{}'::jsonb"],["workspace_id","varchar",true]],"workspace":[["id","varchar",true],["name","varchar",true],["owner","varchar",true],["deleted","bool",true,"false"],["premium","bool",true,"false"]],"magic_link":[["email","varchar",true],["token","varchar",true],["expiration","timestamptz",true,"(now() + '1 day'::interval)"]],"resume_job":[["id","uuid",true],["job","uuid",true],["flow","uuid",true],["value","jsonb",true,"'null'::jsonb"],["approver","varchar",false],["resume_id","int4",true,"0"],["created_at","timestamptz",true,"now()"]],"app_version":[["id","int8",true,"nextval('app_version_id_seq'::regclass)"],["value","json",true],["app_id","int8",true],["created_at","timestamptz",true,"now()"],["created_by","varchar",true]],"worker_ping":[["ip","varchar",true,"'NO IP'::character varying"],["worker","varchar",true],["ping_at","timestamptz",true,"now()"],["started_at","timestamptz",true,"now()"],["jobs_executed","int4",true,"0"],["worker_instance","varchar",true]],"usr_to_group":[["usr","varchar",true,"'ruben'::character varying"],["group_","varchar",true],["workspace_id","varchar",true]],"completed_job":[["id","uuid",true],["tag","varchar",true,"'other'::character varying"],["args","jsonb",false],["logs","text",false],["email","varchar",true,"'missing@email.xyz'::character varying"],["env_id","int4",true,"0"],["result","jsonb",false],["deleted","bool",true,"false"],["success","bool",true],["canceled","bool",true,"false"],["job_kind","job_kind",true,"'script'::job_kind"],["language","script_lang",false,"'python3'::script_lang"],["mem_peak","int4",false],["raw_code","text",false],["raw_flow","jsonb",false],["raw_lock","text",false],["created_at","timestamptz",true],["created_by","varchar",true],["is_skipped","bool",true,"false"],["parent_job","uuid",false],["started_at","timestamptz",true,"now()"],["canceled_by","varchar",false],["duration_ms","int4",true],["flow_status","jsonb",false],["script_hash","int8",false],["script_path","varchar",false],["is_flow_step","bool",false,"false"],["workspace_id","varchar",true],["schedule_path","varchar",false],["canceled_reason","text",false],["permissioned_as","varchar",true,"'g/all'::character varying"],["visible_to_owner","bool",false,"true"]],"resource_type":[["name","varchar",true],["schema","jsonb",false],["description","text",false],["workspace_id","varchar",true]],"workspace_key":[["key","varchar",true,"'changeme'::character varying"],["kind","workspace_key_kind",true],["workspace_id","varchar",true]],"_sqlx_migrations":[["success","bool",true],["version","int8",true],["checksum","bytea",true],["description","text",true],["installed_on","timestamptz",true,"now()"],["execution_time","int8",true]],"workspace_invite":[["email","varchar",true],["is_admin","bool",true,"false"],["operator","bool",true,"false"],["workspace_id","varchar",true]],"workspace_settings":[["plan","varchar",false],["webhook","text",false],["deploy_to","varchar",false],["slack_name","varchar",false],["customer_id","varchar",false],["slack_email","varchar",true,"'missing@email.xyz'::character varying"],["workspace_id","varchar",true],["error_handler","varchar",false],["slack_team_id","varchar",false],["auto_invite_domain","varchar",false],["auto_invite_operator","bool",false,"false"],["openai_resource_path","varchar",false],["slack_command_script","varchar",false]],"pip_resolution_cache":[["hash","varchar",true],["lockfile","text",true],["expiration","timestamp",true]],"databases":[["db_name","text",true],["test_path","text",true],["created_at","timestamptz",true,"now()"]]}"""
