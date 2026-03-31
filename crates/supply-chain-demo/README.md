# Supply Chain & Inventory Management Demo

Demo pack for inventory monitoring, order tracking, and automated reorder management using MCP tools and state management.

## Features

- Real-time stock level monitoring with search and filtering
- Purchase order tracking with status timeline
- Automated low stock alerts with configurable thresholds
- Reorder request creation with approval workflow
- Supplier directory with RFQ capabilities
- Adaptive Card UI for interactive dashboard experience
- State management for session context and workflow continuity

## Supported Messaging Platforms

| Platform | Status |
|----------|--------|
| WebChat GUI | Required |
| Slack | Optional |
| Microsoft Teams | Optional |

## Prerequisites

1. **Inventory API Key**: Obtain an API key for your inventory management system
2. **Greentic CLI**: Install `gtc` CLI tool

## Quick Start

### 1. Build Demo Pack and Create Bundle

```bash
scripts/package_demos.sh
gtc wizard --answers demos/supply-chain-create-answers.json
```

### 2. Setup Providers

```bash
gtc setup ./supply-chain-demo-bundle

# Or with answers file
gtc setup --answers demos/supply-chain-setup-answers.json ./supply-chain-demo-bundle
```

### 3. Start Server

```bash
gtc start ./supply-chain-demo-bundle
```

## Migration Status

This demo is mid-migration to the wizard-generated pack model used by `quickstart-demo`, `cards-demo`, and `hr-onboarding-demo`.

Current source of truth retained in the repo:

- crate assets under [`assets/`](./assets)
- pack build answers in [`gtc_pack_wizard_answers.json`](./gtc_pack_wizard_answers.json)
- existing checked-in source pack under [`bundle/packs/supply-chain.pack`](./bundle/packs/supply-chain.pack)

Captured wizard replay work-in-progress is stored in:

- [`gtc_main_flow_wizard_answers.draft.json`](./gtc_main_flow_wizard_answers.draft.json) for the scaffold default flow that is replacing the old `on_message` path
- [`gtc_pack_create_wizard_answers.draft.json`](./gtc_pack_create_wizard_answers.draft.json) for the first captured `stock_check_flow` replay
- [`gtc_order_tracking_flow_wizard_answers.draft.json`](./gtc_order_tracking_flow_wizard_answers.draft.json) for `order_tracking_flow`
- [`gtc_reorder_flow_wizard_answers.draft.json`](./gtc_reorder_flow_wizard_answers.draft.json) for `reorder_flow`

Those draft files are intentionally not activated in packaging yet because the live consolidated create replay has not been assembled and validated end-to-end.

Current limitation:

- the checked-in source pack still fails `greentic-pack doctor` / `greentic-pack wizard apply` because the existing flow resolve summaries are incomplete
- so this demo is not yet on the fully generated-pack path used by `quickstart-demo`, `cards-demo`, and `hr-onboarding-demo`
- the intended generated shape is now:
  - scaffold `main.ygtc` as the primary entry flow
  - `stock_check_flow`, `order_tracking_flow`, and `reorder_flow` as secondary flows

## Configuration

### Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `INVENTORY_API_KEY` | API key for inventory management system | Yes |
| `SUPPLIER_API_KEY` | API key for supplier portal | No |
| `MCP_INVENTORY_TOOLS_URL` | URL to inventory MCP tools WASM | No (has default) |

### Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `low_stock_threshold` | 10 | Minimum stock level before triggering alerts |
| `reorder_lead_days` | 7 | Default lead time for reorder calculations |
| `currency` | USD | Display currency for pricing |
| `default_warehouse` | WH-001 | Default warehouse for stock queries |
| `items_per_page` | 20 | Number of items per page in listings |

## Pack Structure

```
supply-chain/
├── pack.yaml              # Pack manifest
├── bindings.yaml          # Tenant bindings
├── greentic.demo.yaml     # Demo runtime configuration
├── wizard-answers.yaml    # Wizard answers template
├── flows/
│   ├── on_message.ygtc         # Main message handler & dashboard
│   ├── stock_check_flow.ygtc   # Stock search and detail view
│   ├── order_tracking_flow.ygtc # Order tracking and status
│   └── reorder_flow.ygtc       # Reorder creation and approval
└── assets/
    ├── welcome_card.json        # Dashboard with key metrics
    ├── stock_status_card.json   # Stock table with search
    ├── order_tracking_card.json # Order timeline and list
    ├── reorder_card.json        # Reorder request form
    ├── supplier_card.json       # Supplier directory
    └── alert_card.json          # Low stock alerts
```

## Components

| Component | Purpose |
|-----------|---------|
| `component-adaptive-card` | Render interactive UI cards |
| `mcp.exec` | Execute MCP tools for inventory/order APIs |

## Flows

### Main Entry Flow

Main messaging entry flow. In the migrated wizard-generated model this is the scaffold default flow (`main.ygtc`), replacing the old checked-in `on_message.ygtc`. It loads dashboard metrics (total SKUs, low stock count, pending orders) and routes to sub-flows based on user selection:
- Check Stock -> stock search and detail
- Track Orders -> order list and tracking
- Low Stock Alerts -> items below threshold
- Create Reorder -> reorder form
- Supplier Directory -> supplier browse and RFQ

### stock_check_flow

Search and browse inventory:
1. Search by SKU, product name, or category
2. Filter by warehouse and category
3. View item detail with stock breakdown
4. View stock movement history
5. Quick reorder from detail view

### order_tracking_flow

Track purchase orders and shipments:
1. Search by order ID or browse recent orders
2. Filter by status (Pending, Confirmed, Shipped, In Transit, Delivered)
3. View order detail with status timeline
4. Contact supplier about an order
5. Cancel order with confirmation

### reorder_flow

Create and manage reorder requests:
1. View low stock items needing reorder
2. Fill reorder form (product, quantity, supplier, urgency)
3. Validate input fields
4. Submit reorder request via MCP
5. Review pending approvals
6. Approve or reject reorder requests

## MCP Tools

| Tool | Description |
|------|-------------|
| `inventory_check_stock` | Check stock level for a specific SKU |
| `inventory_search_products` | Search products by name, category, or warehouse |
| `inventory_get_low_stock` | Get all items below the configured threshold |
| `order_create` | Create a new purchase order |
| `order_get_status` | Get order status and tracking details |
| `order_list` | List orders with optional filters |
| `supplier_get_list` | Get supplier directory listing |
| `supplier_send_rfq` | Send request for quote to a supplier |
| `reorder_create` | Create a new reorder request |
| `reorder_approve` | Approve or reject a reorder request |

## Deployment

### Local Development

```bash
gtc start ./supply-chain --cloudflared off
```

### With Tunnel

```bash
gtc start ./supply-chain --cloudflared on
```

## License

MIT
