# invoice-approval

Scripted Adaptive Card demo for invoice intake, validation, discrepancy review, and approval escalation.

This pack is intentionally deterministic and keeps all scenario data in the checked-in card JSON files under `assets/cards/`.

For maintainers:

- `scripts/scenario-map.json` documents the step order and submit-action routing.
- `scripts/validate.sh` checks the published filenames and bundle naming.
- Regenerate the package artifact with `scripts/package_demos.sh` from the repository root.
