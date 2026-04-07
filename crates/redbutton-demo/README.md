# redbutton-demo

This demo bundle is scaffolded for:

- `gtc wizard --answers demos/redbutton-create-answers.json`
- `gtc setup --answers demos/redbutton-setup-answers.json`
- `gtc start`

It wires a global webhook event into a red button scenario that selects a random branch, prepares local audio playback, optionally calls an HTTP endpoint, and creates a Better Stack incident using the `betterstack_token` secret.

Testing webhook:

```bash
curl -X POST http://127.0.0.1:8080/v1/events/ingress/webhook/demo/default \
  -H 'content-type: application/json' \
  -d '{"source":"red-button","pressed":true}'
```
