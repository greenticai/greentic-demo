locals {
  name = "${var.project}-${var.environment}"
}

resource "azurerm_resource_group" "runner" {
  name     = var.resource_group
  location = var.location
}

resource "azurerm_log_analytics_workspace" "runner" {
  name                = "${local.name}-logs"
  location            = azurerm_resource_group.runner.location
  resource_group_name = azurerm_resource_group.runner.name
  sku                 = "PerGB2018"
  retention_in_days   = 30
}

resource "azurerm_container_app_environment" "runner" {
  name                       = "${local.name}-env"
  location                   = azurerm_resource_group.runner.location
  resource_group_name        = azurerm_resource_group.runner.name
  log_analytics_workspace_id = azurerm_log_analytics_workspace.runner.id
}

resource "azurerm_user_assigned_identity" "ci" {
  name                = "${local.name}-ci"
  location            = azurerm_resource_group.runner.location
  resource_group_name = azurerm_resource_group.runner.name
}

resource "azurerm_federated_identity_credential" "github" {
  name                = "github-actions"
  resource_group_name = azurerm_resource_group.runner.name
  parent_id           = azurerm_user_assigned_identity.ci.id
  audience            = ["api://AzureADTokenExchange"]
  issuer              = "https://token.actions.githubusercontent.com"
  subject             = "repo:greenticai/greentic-demo:ref:refs/heads/main"
}

resource "azurerm_container_app" "runner" {
  name                         = local.name
  container_app_environment_id = azurerm_container_app_environment.runner.id
  resource_group_name          = azurerm_resource_group.runner.name
  revision_mode                = "Single"

  ingress {
    external_enabled = true
    target_port      = 8080
  }

  template {
    container {
      name   = "runner"
      image  = var.image
      cpu    = 0.5
      memory = "1Gi"
      env {
        name  = "PACK_SOURCE"
        value = "fs"
      }
      env {
        name  = "PACK_INDEX_URL"
        value = var.pack_index_url
      }
      env {
        name  = "PACK_CACHE_DIR"
        value = var.pack_cache_dir
      }
      env {
        name  = "SECRETS_BACKEND"
        value = var.secrets_backend
      }
      env {
        name  = "TENANT_RESOLVER"
        value = var.tenant_resolver
      }
      env {
        name  = "PACK_REFRESH_INTERVAL"
        value = var.pack_refresh_interval
      }
      env {
        name  = "OTEL_EXPORTER_OTLP_ENDPOINT"
        value = var.telemetry_endpoint
      }
      env {
        name  = "OTEL_SERVICE_NAME"
        value = var.otel_service_name
      }
      dynamic "env" {
        for_each = var.pack_public_key == null ? [] : [var.pack_public_key]
        content {
          name  = "PACK_PUBLIC_KEY"
          value = env.value
        }
      }
    }
    scale {
      min_replicas = 1
      max_replicas = 2
    }
  }

  identity {
    type         = "UserAssigned"
    identity_ids = [azurerm_user_assigned_identity.ci.id]
  }
}

