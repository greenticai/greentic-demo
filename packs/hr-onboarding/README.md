# HR Onboarding Demo

Demo pack for employee onboarding with MCP (Model Context Protocol) tools, document collection, and access provisioning.

## Features

- Step-by-step employee onboarding wizard (name, role, department, start date, manager)
- Onboarding checklist with task tracking and progress visualization
- Document collection and verification (ID, tax forms, contracts)
- System access provisioning (email, VPN, Slack, GitHub, Jira, building access)
- Overall onboarding progress tracking
- Adaptive Card UI for interactive experience
- State management for onboarding session persistence

## Supported Messaging Platforms

| Platform | Status |
|----------|--------|
| WebChat GUI | Required |
| Slack | Optional |
| Microsoft Teams | Optional |

## Prerequisites

1. **HR System API Key**: Obtain an API key for your HR management system
2. **Greentic CLI**: Install `gtc` CLI tool

## Quick Start

### 1. Create Bundle from Wizard

```bash
gtc wizard --answers wizard-answers.yaml
```

### 2. Setup Providers

```bash
# Interactive setup
gtc setup ./hr-onboarding

# Or with answers file
gtc setup --answers setup-answers.json ./hr-onboarding
```

### 3. Start Server

```bash
gtc start ./hr-onboarding
```

## Configuration

### Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `HR_SYSTEM_API_KEY` | API key for HR management system | Yes |
| `MCP_HR_TOOLS_URL` | URL to HR MCP tools WASM | No (has default) |

### Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `company_name` | Acme Corp | Company name shown in onboarding cards |
| `onboarding_checklist_version` | v2 | Checklist template version |
| `default_department` | Engineering | Default department for new hires |
| `document_retention_days` | 90 | Days to retain uploaded documents |

## Pack Structure

```
hr-onboarding/
├── pack.yaml                      # Pack manifest
├── bindings.yaml                  # Tenant bindings
├── greentic.demo.yaml             # Demo server configuration
├── wizard-answers.yaml            # Wizard answers template
├── flows/
│   ├── on_message.ygtc            # Main message handler
│   ├── onboarding_wizard_flow.ygtc    # Employee info collection wizard
│   ├── document_collection_flow.ygtc  # Document upload and tracking
│   └── access_provisioning_flow.ygtc  # System access requests
└── assets/
    ├── welcome_card.json              # HR onboarding menu
    ├── employee_form_card.json        # Employee information form
    ├── onboarding_checklist_card.json # Checklist with progress
    ├── document_upload_card.json      # Document status and upload
    ├── access_request_card.json       # System access toggles
    └── completion_card.json           # Onboarding complete summary
```

## Components

| Component | Purpose |
|-----------|---------|
| `component-adaptive-card` | Render interactive UI cards |
| `mcp.exec` | Execute MCP tools for HR system API |

## Flows

### on_message

Main entry point for messaging. Checks onboarding state and routes to the appropriate sub-flow (wizard, documents, access, or progress check).

### onboarding_wizard_flow

Collects employee information through a multi-step wizard:
1. Show employee form card
2. Validate submitted data
3. Create employee record via MCP
4. Fetch onboarding checklist for role/department
5. Display checklist and save onboarding state

### document_collection_flow

Manages required document submissions:
1. Fetch current document status via MCP
2. Display document upload card with per-item status
3. Handle document submission
4. Update document status
5. Check if all documents are collected

### access_provisioning_flow

Handles system access requests:
1. Display access request card with available systems
2. Validate selected access items
3. Provision access via MCP
4. Show provisioning status
5. Update onboarding checklist

## MCP Tools

| Tool | Description |
|------|-------------|
| `hr_create_employee` | Register new employee in HR system |
| `hr_get_checklist` | Get onboarding checklist for role/department |
| `hr_update_checklist` | Mark checklist items as complete |
| `hr_submit_document` | Submit/register a document |
| `hr_get_document_status` | Check document submission status |
| `hr_provision_access` | Request system access |
| `hr_get_access_status` | Check access provisioning status |
| `hr_get_onboarding_progress` | Get overall onboarding progress |

## Onboarding Workflow

```
User sends message
    │
    ├─ No onboarding state → Show Welcome Card
    │   ├─ "Start Onboarding" → Onboarding Wizard Flow
    │   ├─ "Check Progress"   → Show Progress
    │   ├─ "Upload Documents" → Document Collection Flow
    │   └─ "Request Access"   → Access Provisioning Flow
    │
    └─ Has onboarding state → Show Welcome Card (with progress)
        ├─ Resume incomplete steps
        └─ Show completion card when all done
```

## Deployment

### Local Development

```bash
gtc start ./hr-onboarding --cloudflared off
```

### AWS Deployment

```bash
gtc start ./hr-onboarding --deploy aws
```

## License

MIT
