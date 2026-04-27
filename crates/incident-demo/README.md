# incident-demo

> Work in progress: this demo is not currently listed in the top-level demo catalog while the incident flow is still being stabilized.

This demo bundle is scaffolded for:

- `gtc wizard --answers demos/incident-create-answers.json`
- `gtc setup --answers demos/incident-setup-answers.json`
- `gtc start`

It is a best-effort messaging demo that targets WebChat and WebEx, renders an adaptive-card incident creation screen, and uses the `betterstack_token` secret to create a Better Stack incident.

Notes:

- The WebChat and WebEx provider references are the exact selections from `gtc wizard` common messaging providers.
- The adaptive-card interaction flow is best-effort because this repo does not contain an existing WebChat/WebEx messaging example to mirror.
- The Better Stack component reuses the existing `component-betterstack-incident` artifact from `redbutton-demo`.
