# greentic-demo

Runnable Greentic demo catalog.

This repository provides prebuilt answer documents per demo so you can launch each one with the same 3-step flow:

1. `gtc wizard --answers https://github.com/greenticai/greentic-demo/releases/latest/download/<demo>-create-answers.json`
2. `gtc setup <bundle> --answers https://github.com/greenticai/greentic-demo/releases/latest/download/<demo>-setup-answers.json`
3. `gtc start <bundle>`

## Available Demos

The demos below have both `*-create-answers.json` and `*-setup-answers.json` published in releases and are runnable with the 3-step flow.
Standalone app-pack artifacts may also be published alongside these demos for reuse in other bundle flows.

### quickstart

Outcome:
- Starts a minimal assistant that shows a welcome card, an about card, and basic chat interactions.

Run:
```bash
gtc wizard --answers https://github.com/greenticai/greentic-demo/releases/latest/download/quickstart-create-answers.json
gtc setup ./quickstart-demo-bundle --answers https://github.com/greenticai/greentic-demo/releases/latest/download/quickstart-setup-answers.json
gtc start ./quickstart-demo-bundle
```

### hr-onboarding

Outcome:
- Runs an onboarding assistant for employee intake, checklist tracking, and document/access collection.

Run:
```bash
gtc wizard --answers https://github.com/greenticai/greentic-demo/releases/latest/download/hr-onboarding-create-answers.json
gtc setup ./hr-onboarding-demo-bundle --answers https://github.com/greenticai/greentic-demo/releases/latest/download/hr-onboarding-setup-answers.json
gtc start ./hr-onboarding-demo-bundle
```

### helpdesk-itsm

Outcome:
- Runs an IT helpdesk assistant with Jira-oriented ticket flows (create, status, escalation, KB lookup).

Run:
```bash
gtc wizard --answers https://github.com/greenticai/greentic-demo/releases/latest/download/helpdesk-itsm-create-answers.json
gtc setup ./helpdesk-itsm-demo-bundle --answers https://github.com/greenticai/greentic-demo/releases/latest/download/helpdesk-itsm-setup-answers.json
gtc start ./helpdesk-itsm-demo-bundle
```

### sales-crm

Outcome:
- Runs a sales assistant for lead qualification, pipeline visibility, and deal tracking.

Run:
```bash
gtc wizard --answers https://github.com/greenticai/greentic-demo/releases/latest/download/sales-crm-create-answers.json
gtc setup ./sales-crm-demo-bundle --answers https://github.com/greenticai/greentic-demo/releases/latest/download/sales-crm-setup-answers.json
gtc start ./sales-crm-demo-bundle
```

### supply-chain

Outcome:
- Runs an inventory/supply-chain assistant for stock checks, order tracking, and reorder workflows.

Run:
```bash
gtc wizard --answers https://github.com/greenticai/greentic-demo/releases/latest/download/supply-chain-create-answers.json
gtc setup ./supply-chain-demo-bundle --answers https://github.com/greenticai/greentic-demo/releases/latest/download/supply-chain-setup-answers.json
gtc start ./supply-chain-demo-bundle
```

### incident

Outcome:
- Runs an incident flow demo with adaptive-card collection and Better Stack incident creation.

Run:
```bash
gtc wizard --answers https://github.com/greenticai/greentic-demo/releases/latest/download/incident-create-answers.json
gtc setup ./incident-demo-bundle --answers https://github.com/greenticai/greentic-demo/releases/latest/download/incident-setup-answers.json
gtc start ./incident-demo-bundle
```

### redbutton

Outcome:
- Runs a red-button response scenario that routes inbound events and triggers branch actions and incident hooks.

Run:
```bash
gtc wizard --answers https://github.com/greenticai/greentic-demo/releases/latest/download/redbutton-create-answers.json
gtc setup ./redbutton-demo-bundle --answers https://github.com/greenticai/greentic-demo/releases/latest/download/redbutton-setup-answers.json
gtc start ./redbutton-demo-bundle
```

### cloud-deploy-demo

Outcome:
- Runs a deployment-focused demo bundle that includes messaging, events, state, and deploy-provider wiring.

Run:
```bash
gtc wizard --answers https://github.com/greenticai/greentic-demo/releases/latest/download/cloud-deploy-demo-create-answers.json
gtc setup ./cloud-deploy-demo-bundle --answers https://github.com/greenticai/greentic-demo/releases/latest/download/cloud-deploy-demo-setup-answers.json
gtc start ./cloud-deploy-demo-bundle
```
