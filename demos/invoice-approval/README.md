# invoice-approval

Maintenance notes for the scripted invoice approval demo.

Source pack:

- `crates/invoice-approval-demo/bundle/packs/invoice-approval.pack/`

Scenario files:

- `assets/cards/*.json` contains the Adaptive Card payloads for the selector, valid invoice, discrepancy, and high-value paths.
- `scripts/scenario-map.json` documents the assistant message shown before each card and the deterministic action routing.
- `scripts/validate.sh` checks the published filenames, bundle naming, and local release paths.

Regenerate the pack:

```bash
scripts/package_demos.sh
```

Safe card-copy updates:

- Keep `version` at `1.5` for all cards.
- Keep every `Action.Submit` payload explicit with `scenario`, `step`, and `action`.
- Update `scripts/scenario-map.json` and `flows/main.ygtc` together so the demo stays deterministic.
