locals {
  name = "${var.project}-${var.environment}"
}

data "google_project" "current" {
  project_id = var.project
}

resource "google_service_account" "runner" {
  account_id   = "${var.environment}-runner"
  display_name = "Runner host"
  project      = var.project
}

resource "google_iam_workload_identity_pool" "github" {
  project      = var.project
  location     = "global"
  workload_identity_pool_id = "github-${var.environment}"
  display_name = "GitHub Actions"
}

resource "google_iam_workload_identity_pool_provider" "github" {
  project                            = var.project
  location                           = "global"
  workload_identity_pool_id          = google_iam_workload_identity_pool.github.workload_identity_pool_id
  workload_identity_pool_provider_id = "github"
  display_name                       = "GitHub OIDC"
  attribute_mapping = {
    "google.subject"           = "assertion.sub"
    "attribute.repository"     = "assertion.repository"
    "attribute.ref"            = "assertion.ref"
  }
  oidc {
    issuer_uri = "https://token.actions.githubusercontent.com"
  }
}

resource "google_service_account_iam_binding" "runner_wi" {
  service_account_id = google_service_account.runner.id
  role               = "roles/iam.workloadIdentityUser"
  members = [
    "principalSet://iam.googleapis.com/${google_iam_workload_identity_pool.github.name}/attribute.repository/greenticai/greentic-demo"
  ]
}

resource "google_cloud_run_service" "runner" {
  name     = local.name
  project  = var.project
  location = var.region

  template {
    metadata {
      annotations = {
        "autoscaling.knative.dev/minScale" = "1"
      }
    }
    spec {
      service_account_name = google_service_account.runner.email
      containers {
        image = var.image
        ports {
          name = "http1"
          container_port = 8080
        }
        env = concat([
          { name = "PACK_SOURCE",           value = "fs" },
          { name = "PACK_INDEX_URL",        value = var.pack_index_url },
          { name = "PACK_CACHE_DIR",        value = var.pack_cache_dir },
          { name = "SECRETS_BACKEND",       value = var.secrets_backend },
          { name = "TENANT_RESOLVER",       value = var.tenant_resolver },
          { name = "PACK_REFRESH_INTERVAL", value = var.pack_refresh_interval },
          { name = "OTEL_EXPORTER_OTLP_ENDPOINT", value = var.telemetry_endpoint },
          { name = "OTEL_SERVICE_NAME", value = var.otel_service_name }
        ],
        var.pack_public_key == null ? [] : [
          { name = "PACK_PUBLIC_KEY", value = var.pack_public_key }
        ])
      }
    }
  }

  traffic {
    percent         = 100
    latest_revision = true
  }
}

