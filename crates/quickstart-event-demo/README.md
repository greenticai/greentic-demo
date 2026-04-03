# Quickstart Event Demo

Demonstrates all Greentic event providers working together:

- **Webhook** — Receive HTTP POST events from external services
- **Timer** — Cron-scheduled events
- **Email (SendGrid)** — Send transactional emails
- **SMS (Twilio)** — Send SMS messages

## Quick Start

```bash
# Create bundle
gtc wizard --answers https://github.com/greenticai/greentic-demo/releases/latest/download/quickstart-event-create-answers.json

# Setup providers
gtc setup ./quickstart-event-demo-bundle --answers https://github.com/greenticai/greentic-demo/releases/latest/download/quickstart-event-setup-answers.json

# Start
gtc start ./quickstart-event-demo-bundle --ngrok on
```

## Test Webhook

```bash
curl -X POST https://<ngrok-url>/events/webhook/demo/alert \
  -H 'Content-Type: application/json' \
  -d '{"message": "Hello from webhook!", "source": "test"}'
```

## Providers

| Provider | OCI Reference | Required Secrets |
|----------|--------------|-----------------|
| Webhook | `events-webhook` | None (optional: secret_key for HMAC validation) |
| Timer | `events-timer` | None |
| Email | `events-email-sendgrid` | `sendgrid_api_key`, `from_email` |
| SMS | `events-sms-twilio` | `account_sid`, `auth_token`, `from_number` |
