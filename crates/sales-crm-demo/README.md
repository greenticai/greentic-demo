# Sales CRM Assistant Demo

Demo pack for Salesforce OAuth integration with MCP tools for lead qualification, pipeline management, and deal tracking.

## Features

- Salesforce OAuth authentication flow
- Lead qualification with scoring (company, budget, timeline, authority)
- Sales pipeline visualization by stage
- Deal tracking with amount, stage, and close date management
- Meeting scheduling with contacts and leads
- Adaptive Card UI for interactive CRM experience
- **msg2events**: Convert messages to Salesforce events
- **events2msg**: Handle Salesforce outbound messages and convert to chat notifications

## Supported Messaging Platforms

| Platform | Status |
|----------|--------|
| WebChat GUI | Required |
| Slack | Optional |
| Microsoft Teams | Optional |
| Webex | Optional |

## Prerequisites

1. **Salesforce Connected App**: Create a Connected App in Salesforce Setup
   - Navigate to Setup > App Manager > New Connected App
   - Enable OAuth Settings
   - Set callback URL to: `{your_domain}/oauth/callback/{tenant}/salesforce`
   - Select OAuth scopes: `api`, `refresh_token`
   - Note the Consumer Key (Client ID) and Consumer Secret (Client Secret)

2. **Greentic CLI**: Install `gtc` CLI tool

## Quick Start

### 1. Create Bundle from Wizard

```bash
gtc wizard --answers wizard-answers.yaml
```

### 2. Setup Providers

```bash
# Interactive setup
gtc setup ./sales-crm

# Or with answers file
gtc setup --answers setup-answers.json ./sales-crm
```

### 3. Start Server

```bash
gtc start ./sales-crm
```

## Configuration

### Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `SALESFORCE_CLIENT_ID` | Salesforce Connected App Consumer Key | Yes |
| `SALESFORCE_CLIENT_SECRET` | Salesforce Connected App Consumer Secret | Yes |
| `MCP_CRM_TOOLS_URL` | URL to CRM MCP tools WASM | No (has default) |

### OAuth Scopes

The following Salesforce OAuth scopes are requested:

- `api` - Access and manage your Salesforce data
- `refresh_token` - Perform requests at any time (refresh token)

## Pack Structure

```
sales-crm/
├── pack.yaml                        # Pack manifest
├── bindings.yaml                    # Tenant bindings
├── greentic.demo.yaml               # Demo configuration
├── wizard-answers.yaml              # Wizard answers template
├── flows/
│   ├── on_message.ygtc              # Main message handler
│   ├── on_event.ygtc                # Salesforce webhook event handler
│   ├── salesforce_oauth_flow.ygtc   # OAuth flow
│   ├── lead_qualification_flow.ygtc # Lead qualification flow
│   └── deal_tracker_flow.ygtc       # Deal tracking and pipeline flow
└── assets/
    ├── welcome_card.json            # CRM dashboard menu card
    ├── salesforce_connect_card.json # OAuth connection card
    ├── lead_form_card.json          # Lead capture form card
    ├── pipeline_card.json           # Sales pipeline stages card
    ├── deal_detail_card.json        # Deal details card
    └── meeting_card.json            # Meeting scheduler card
```

## Components

| Component | Purpose |
|-----------|---------|
| `component-msg2events` | Convert user messages to Salesforce events |
| `component-events2msg` | Transform Salesforce webhooks to chat messages |
| `component-adaptive-card` | Render interactive UI cards |
| `mcp.exec` | Execute MCP tools for Salesforce API |

## Flows

### on_message

Main entry point for messaging. Checks Salesforce authentication state and routes to CRM features (lead qualification, pipeline view, deal tracking, meeting scheduling).

### salesforce_oauth_flow

Handles Salesforce OAuth authorization:
1. Show Salesforce connect card
2. Redirect to Salesforce login
3. Handle callback
4. Exchange code for access token
5. Save token and fetch user identity
6. Confirm successful connection

### lead_qualification_flow

Lead capture and qualification:
1. Show lead form (company, contact, budget, timeline)
2. Validate inputs
3. Create lead in Salesforce via MCP
4. Score and qualify the lead
5. Display qualification results
6. Route to pipeline or schedule follow-up

### deal_tracker_flow

Pipeline management and deal tracking:
1. Fetch pipeline opportunities from Salesforce
2. Display pipeline stages (Prospecting through Closed Won)
3. Select a deal for detail view
4. Update deal stage, amount, or notes
5. Confirm changes

### on_event

Handle incoming Salesforce outbound messages:
- Lead Created: Notify team of new lead
- Opportunity Updated: Notify of stage changes
- Deal Closed Won: Celebrate and notify team
- Deal Closed Lost: Notify with loss reason

## MCP Tools

The pack uses the following MCP tools:

| Tool | Description |
|------|-------------|
| `sf_create_lead` | Create a new lead in Salesforce |
| `sf_get_lead` | Get lead details by ID |
| `sf_qualify_lead` | Score and qualify a lead |
| `sf_get_pipeline` | Get sales pipeline (opportunities by stage) |
| `sf_get_opportunity` | Get opportunity/deal details |
| `sf_update_opportunity` | Update opportunity stage, amount, or notes |
| `sf_create_task` | Create a task or meeting |
| `sf_search` | Execute SOQL search queries |

## Deployment

### AWS Deployment

```bash
gtc start ./sales-crm --deploy aws
```

### Local Development

```bash
gtc start ./sales-crm --cloudflared on
```

## License

MIT
