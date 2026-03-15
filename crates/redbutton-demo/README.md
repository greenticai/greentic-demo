# redbutton-demo

This demo bundle is scaffolded for:

- `gtc wizard --answers demos/redbutton-create-answers.json`
- `gtc setup --answers demos/redbutton-setup-answers.json`
- `gtc start`

It wires a global webhook event into a red button scenario that selects a random branch, prepares local audio playback, optionally calls an HTTP endpoint, and creates a Better Stack incident using the `betterstack_token` secret.

Testing webhook:

```bash
curl -X POST http://127.0.0.1:8080/events/redbutton \
  -H 'content-type: application/json' \
  -d '{"source":"red-button","pressed":true}'
```

Current toolchain note:

- `greentic-bundle wizard apply --answers demos/redbutton-create-answers.json` validates and materializes the bundle workspace.
- On this machine, `gtc wizard --answers ...` still routes to `greentic-dev wizard`, which does not accept `--answers`.
- On this machine, `gtc setup --answers ...` still expects `greentic.demo.yaml` instead of a `bundle.yaml` workspace, so the checked-in setup answers document is preparatory rather than executable until that router/setup mismatch is fixed upstream.
