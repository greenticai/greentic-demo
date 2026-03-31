# quickstart-demo

Minimal Greentic demo that displays a welcome card with simple menu interactions.

## Features

- Welcome card with platform info
- Echo message demonstration
- About page with Greentic platform details

## Quick Start

```bash
scripts/package_demos.sh
gtc wizard --answers demos/quickstart-create-answers.json
gtc setup --answers demos/quickstart-setup-answers.json ./quickstart-demo-bundle
gtc start ./quickstart-demo-bundle
```

## Components

| Component | Purpose |
|-----------|---------|
| `component-adaptive-card` | Render interactive UI cards |

## License

MIT
