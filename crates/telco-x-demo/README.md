# Telco-X Demo

Telco-X messaging demo for Webchat GUI with category menus and telco playbook flows.

## Package

```bash
bash scripts/package_demos.sh telco-x-demo
```

## Run

```bash
gtc wizard --answers demos/telco-x-demo-create-answers.json
gtc setup --answers demos/telco-x-demo-setup-answers.json ./telco-x-demo-bundle
gtc start ./telco-x-demo-bundle
```

## Webchat

Open the URL printed by `gtc start`.

For the default local run, it looks like:

```text
http://127.0.0.1:8080/v1/web/webchat/demo/
```

Try:

- `show overutilised aci ports`
- `show recent change correlation`
- `run vm rca`
- `investigate service degradation`
