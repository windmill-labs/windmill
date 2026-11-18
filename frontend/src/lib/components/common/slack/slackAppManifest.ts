/** Requested on top of the usual ones when a workspace connects with AI agents enabled. */
export const SLACK_AGENT_SCOPES = ['users:read', 'users:read.email', 'channels:read']

/**
 * The Slack app manifest Windmill expects, for `baseUrl` (the instance's public URL). It declares
 * the agent scopes too: an app may declare more than a connection asks for, never fewer.
 */
export function slackAppManifest(baseUrl: string): string {
	return `display_information:
  name: Windmill
  description: Run Windmill scripts, flows and AI agents from Slack
  background_color: '#3b82f6'
features:
  app_home:
    home_tab_enabled: true
    messages_tab_enabled: true
    messages_tab_read_only_enabled: false
  bot_user:
    display_name: Windmill
    always_online: true
  slash_commands:
    - command: /windmill
      url: ${baseUrl}/api/oauth/slack_command
      description: Trigger the script set in your workspace settings for Slack
      usage_hint: the text that will be passed to the script
      should_escape: false
oauth_config:
  redirect_urls:
    - ${baseUrl}/oauth/callback_slack
    - ${baseUrl}/oauth/callback_slack/instance
  scopes:
    bot:
      - chat:write
      - chat:write.public
      - channels:join
      - files:write
      - commands
      - app_mentions:read
      - im:history
      - im:read
${SLACK_AGENT_SCOPES.map((s) => `      - ${s}`).join('\n')}
settings:
  event_subscriptions:
    request_url: ${baseUrl}/api/oauth/slack_events
    bot_events:
      - app_mention
      - message.im
  interactivity:
    is_enabled: true
    request_url: ${baseUrl}/api/slack
  org_deploy_enabled: false
  socket_mode_enabled: false
  token_rotation_enabled: false
`
}
