# deep-research-demo

```bash
./scripts/package_demos.sh deep-research-demo
gtc wizard --answers demos/deep-research-demo-create-answers.json
gtc setup deep-research-demo-bundle --answers demos/deep-research-demo-setup-answers.json
gtc start deep-research-demo-bundle
```

## Packaging

- Standard demo build entrypoint: `./scripts/package_demos.sh deep-research-demo`.
- Bundle creation answers live in `demos/deep-research-demo-create-answers.json`.
- Bundle setup answers live in `demos/deep-research-demo-setup-answers.json`.
- Pack build answers live in `build-answer.json`.
- Pack setup prompts live in `assets/setup.yaml`.
- The demo is configured for local Ollama by default, uses `llama3:8b` as the known-good default model, and does not require an API key secret.
- `gtc setup` captures the OpenAI-compatible base URL and preferred model, and the LLM nodes now build their `component.exec` payload explicitly through `in_map` with metadata overrides plus local Ollama fallbacks.
- The messaging flow is generated from `build-answer.json`, including adaptive-card submit routing by `action` metadata into the planner and analyst LLM steps.
