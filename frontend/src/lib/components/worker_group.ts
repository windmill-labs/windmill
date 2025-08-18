// enabled: bool,
// min_workers: usize,
// max_workers: usize,
// cooldown_seconds: usize,
// inc_scale_jobs_waiting: usize,
// full_scale_cooldown_seconds: usize,
// full_scale_jobs_waiting: usize,
// dec_scale_occupancy_rate: u8, // occupancy rate of 30s, 5m, 30m to scale down
// inc_scale_occupancy_rate: u8, // occupancy rate of 30s, 5m to scale up
// inc_percent: usize,
// integration: Option<AutoscalingIntegration>,

export type AutoscalingConfig = {
    enabled: boolean
    min_workers: number
    max_workers: number
    cooldown_seconds?: number
    inc_scale_num_jobs_waiting?: number
    full_scale_cooldown_seconds?: number
    full_scale_jobs_waiting?: number
    dec_scale_occupancy_rate?: number
    inc_scale_occupancy_rate?: number
    inc_num_workers?: number
    integration?: AutoscalingIntegration
    custom_tags?: string[]
}


export type AutoscalingIntegration = AutoscaleScript | AutoscaleDryRun

export type AutoscaleScript = {
    type: 'script'
    tag?: string
    path: string
}

export type AutoscaleDryRun = {
    type: 'dryrun'
}

export const defaultTags = [
    'deno',
    'python3',
    'go',
    'bash',
    'powershell',
    'dependency',
    'flow',
    'other',
    'bun',
    'php',
    'rust',
    'ansible',
    'csharp',
    'nu',
    'java',
    'ruby'
    // for related places search: ADD_NEW_LANG 

]
export const nativeTags = [
    'nativets',
    'postgresql',
    'mysql',
    'graphql',
    'snowflake',
    'mssql',
    'bigquery',
    'oracledb'
]
