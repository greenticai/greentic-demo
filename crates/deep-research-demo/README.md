# deep-research-demo

```bash
./crates/deep-research-demo/build_pack.sh
gtc wizard --answers demos/deep-research-demo-create-answers.json
gtc setup deep-research-demo-bundle --answers demos/deep-research-demo-setup-answers.json
gtc start deep-research-demo-bundle
```

## Packaging

- Dynamic pack generation entrypoint: `./build_pack.sh`.
- Bundle creation answers live in `demos/deep-research-demo-create-answers.json`.
- Bundle setup answers live in `demos/deep-research-demo-setup-answers.json`.
- Pack scaffold answers live in `pack_answers.json`.
- Pack setup prompts live in `assets/setup.yaml`.
