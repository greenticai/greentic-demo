# IT Helpdesk ITSM Demo

Demo pack for IT Helpdesk / ITSM integration with Jira via OAuth and MCP (Model Context Protocol) tools.

## Features

- Jira OAuth authentication flow (Atlassian OAuth 2.0)
- Create support tickets with priority, category, and description
- Check ticket status by issue key
- Search internal knowledge base articles
- Escalate tickets to higher support tiers
- View and manage your open tickets
- **msg2events**: Convert messages to Jira webhook events
- **events2msg**: Handle Jira webhooks and convert to notification messages

## Supported Messaging Platforms

| Platform | Status |
|----------|--------|
| WebChat GUI | Required |
| Slack | Optional |
| Microsoft Teams | Optional |

## Prerequisites

1. **Jira (Atlassian) OAuth App**: Create an OAuth 2.0 integration at https://developer.atlassian.com/console/myapps/
   - Set callback URL to: `{your_domain}/oauth/callback/{tenant}/jira`
   - Enable scopes: `read:jira-work`, `write:jira-work`, `read:jira-user`
   - Note the Client ID and Client Secret

2. **Jira Project**: Create or identify a Jira project (default key: `ITSM`)

3. **Greentic CLI**: Install `gtc` CLI tool

## Quick Start

### 1. Create Bundle from Wizard

```bash
gtc wizard --answers wizard-answers.yaml
```

### 2. Setup Providers

```bash
# Interactive setup
gtc setup ./helpdesk-itsm

# Or with answers file
gtc setup --answers setup-answers.json ./helpdesk-itsm
```

### 3. Start Server

```bash
gtc start ./helpdesk-itsm
```

## Configuration

### Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `JIRA_CLIENT_ID` | Atlassian OAuth App Client ID | Yes |
| `JIRA_CLIENT_SECRET` | Atlassian OAuth App Client Secret | Yes |
| `JIRA_CLOUD_ID` | Atlassian Cloud site ID | No |
| `MCP_HELPDESK_TOOLS_URL` | URL to Helpdesk MCP tools WASM | No (has default) |

### OAuth Scopes

The following Atlassian OAuth scopes are requested:

- `read:jira-work` - Read Jira project and issue data
- `write:jira-work` - Create and edit Jira issues
- `read:jira-user` - Read user profile data

### Jira Project Configuration

The default project key is `ITSM`. To change it, update `parameters.jira_project_key` in `pack.yaml` and the flow files.

## Pack Structure

```
helpdesk-itsm/
├── pack.yaml                   # Pack manifest
├── bindings.yaml               # Tenant bindings
├── greentic.demo.yaml          # Demo runtime configuration
├── wizard-answers.yaml         # Wizard answers template
├── flows/
│   ├── on_message.ygtc         # Main message handler
│   ├── on_event.ygtc           # Jira webhook event handler
│   ├── jira_oauth_flow.ygtc    # OAuth connection flow
│   ├── create_ticket_flow.ygtc # Ticket creation flow
│   └── knowledge_base_flow.ygtc # KB search flow
└── assets/
    ├── welcome_card.json       # Helpdesk menu card
    ├── jira_connect_card.json  # OAuth connection card
    ├── create_ticket_card.json # Ticket creation form
    ├── ticket_status_card.json # Ticket detail/status card
    ├── kb_results_card.json    # Knowledge base results card
    └── escalation_card.json    # Ticket escalation form
```

## Components

| Component | Purpose |
|-----------|---------|
| `component-msg2events` | Convert user messages to Jira events |
| `component-events2msg` | Transform Jira webhooks to chat messages |
| `component-adaptive-card` | Render interactive UI cards |
| `mcp.exec` | Execute MCP tools for Jira API and KB search |

## Flows

### on_message

Main entry point for messaging. Checks Jira OAuth status and routes to the appropriate feature:
1. Create Ticket - open a new Jira issue
2. Check Status - look up a ticket by key
3. Search KB - search the knowledge base
4. My Tickets - view your reported issues
5. Escalate - raise priority on existing tickets

### jira_oauth_flow

Handles Atlassian OAuth authorization:
1. Show connect card with scope details
2. Redirect to Atlassian authorization
3. Handle callback
4. Exchange code for access token
5. Fetch and save user profile info

### create_ticket_flow

Creates new Jira issues:
1. Show form with summary, description, type, priority, category
2. Validate required fields
3. Create issue via MCP tool
4. Show confirmation with ticket key

### knowledge_base_flow

Searches internal knowledge base:
1. Prompt for search query
2. Execute search via MCP tool
3. Display results with relevance scores
4. View full article content
5. Offer to create ticket if KB didn't help

### on_event

Handles incoming Jira webhooks:
- `jira:issue_created` - Notify on new tickets, alert on high priority
- `jira:issue_updated` - Notify on status/field changes, detect resolutions
- `jira:issue_deleted` - Notify on deletions
- `comment_created` / `comment_updated` - Notify on comment activity
- `sprint_started` / `sprint_closed` - Notify on sprint lifecycle

## MCP Tools

The pack uses the following MCP tools:

| Tool | Description |
|------|-------------|
| `jira_create_issue` | Create a new Jira issue |
| `jira_get_issue` | Get issue details by key |
| `jira_search` | Search issues using JQL queries |
| `jira_update_issue` | Update issue fields (priority, labels, etc.) |
| `jira_add_comment` | Add a comment to an issue |
| `jira_get_myself` | Get authenticated user profile |
| `kb_search` | Search knowledge base articles |

## Jira Webhook Setup

To receive Jira events, configure a webhook in your Jira project:

1. Go to Jira Settings > System > WebHooks
2. Create a new webhook pointing to: `{your_domain}/events/webhook/{tenant}/jira`
3. Select events: Issue created, Issue updated, Issue deleted, Comment created

## Deployment

### Local Development

```bash
gtc start ./helpdesk-itsm --cloudflared on
```

### AWS Deployment

```bash
gtc start ./helpdesk-itsm --deploy aws
```

## License

MIT
