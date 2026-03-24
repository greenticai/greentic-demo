# quickstart-demo

Minimal Greentic demo that displays a welcome card with simple menu interactions.

## Features

- Welcome card with platform info
- Echo message demonstration
- About page with Greentic platform details

## Quick Start

```bash
gtc wizard --answers demos/quickstart-create-answers.json
gtc setup --answers demos/quickstart-setup-answers.json crates/quickstart-demo/bundle
gtc start crates/quickstart-demo/bundle
```

## Components

| Component | Purpose |
|-----------|---------|
| `component-adaptive-card` | Render interactive UI cards |

## License

MIT
