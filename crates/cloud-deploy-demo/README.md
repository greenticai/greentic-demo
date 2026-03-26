# cloud-deploy-demo

Greentic demo bundle used to exercise cloud deployment flows with a richer bundle composition.

## Features

- Demo app pack
- Messaging webchat provider
- Event webhook provider
- In-memory state provider
- Terraform deploy provider

## Quick Start

```bash
gtc wizard --answers demos/cloud-deploy-demo-create-answers.json
gtc setup --answers demos/cloud-deploy-demo-setup-answers.json crates/cloud-deploy-demo/bundle
gtc start crates/cloud-deploy-demo/bundle
```

## License

MIT
