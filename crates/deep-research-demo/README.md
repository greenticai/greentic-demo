# deep-research-demo

```bash
./crates/deep-research-demo/build_pack.sh
gtc wizard --answers demos/deep-research-demo-create-answers.json
gtc setup deep-research-demo-bundle --answers demos/deep-research-demo-setup-answers.json
gtc start deep-research-demo-bundle
```

## Packaging

- Dynamic pack generation entrypoint: `./build_pack.sh`.
- `build_pack.sh` generates the flow from `pack_answers.json` via local `greentic-pack`/`greentic-flow` when available, instead of copying a checked-in flow scaffold.
- Bundle creation answers live in `demos/deep-research-demo-create-answers.json`.
- Bundle setup answers live in `demos/deep-research-demo-setup-answers.json`.
- Pack scaffold answers live in `pack_answers.json`.
- Pack setup prompts live in `assets/setup.yaml`.
- The demo is configured for local Ollama by default, uses `llama3:8b` as the known-good default model, and does not require an API key secret.
- `gtc setup` captures the OpenAI-compatible base URL and preferred model, and the LLM nodes now build their `component.exec` payload explicitly through `in_map` with metadata overrides plus local Ollama fallbacks.
- The messaging flow is generated from `pack_answers.json`, including adaptive-card submit routing by `action` metadata into the planner and analyst LLM steps.
