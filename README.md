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
gtc setup --no-ui ./cloud-deploy-demo-bundle --answers https://github.com/greenticai/greentic-demo/releases/latest/download/cloud-deploy-demo-setup-answers.json
gtc start ./cloud-deploy-demo-bundle
```

### weather-mcp-demo

Outcome:
- Runs a weather assistant that fetches current conditions and forecast data, then renders adaptive-card responses.

Run:
```bash
gtc wizard --answers https://github.com/greenticai/greentic-demo/releases/latest/download/weather-mcp-demo-create-answers.json
gtc setup ./weather-mcp-demo-bundle --answers https://github.com/greenticai/greentic-demo/releases/latest/download/weather-mcp-demo-setup-answers.json
gtc start ./weather-mcp-demo-bundle
```

### deep-research-demo

Outcome:
- Runs a deep-research assistant with `Single Shot` and `Agentic` modes, adaptive-card planning, and a final report flow.

Run:
```bash
gtc wizard --answers https://github.com/greenticai/greentic-demo/releases/latest/download/deep-research-demo-create-answers.json
gtc setup ./deep-research-demo-bundle --answers https://github.com/greenticai/greentic-demo/releases/latest/download/deep-research-demo-setup-answers.json
gtc start ./deep-research-demo-bundle
```

Notes:
- By default this demo is configured for a local Ollama endpoint at `http://127.0.0.1:11434/v1` with `llama3:8b`.
- To use Ollama locally, download it from `https://ollama.com/download`, install it, then pull or run the model with `ollama run llama3:8b`.
- If you want to use OpenAI instead, use the OpenAI-compatible base URL `https://api.openai.com/v1` during `gtc setup`.
- You can create or manage your OpenAI API keys at `https://platform.openai.com/api-keys`.
- If you want to use another OpenAI-compatible provider, supply that provider's compatible base URL and API key secret during `gtc setup`.

### telco-x-demo

Outcome:
- Runs a Telco-X assistant in Webchat with category menus, multi-playbook telco flows, and adaptive-card results for traffic, capacity, RCA, and service-assurance scenarios.

Run:
```bash
gtc wizard --answers https://github.com/greenticai/greentic-demo/releases/latest/download/telco-x-demo-create-answers.json
gtc setup ./telco-x-demo-bundle --answers https://github.com/greenticai/greentic-demo/releases/latest/download/telco-x-demo-setup-answers.json
gtc start ./telco-x-demo-bundle
```
