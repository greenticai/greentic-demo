locals {
  name = "${var.project}-${var.environment}"
}

data "aws_caller_identity" "current" {}

data "aws_partition" "current" {}

data "aws_iam_policy_document" "ci_assume" {
  statement {
    effect = "Allow"
    principals {
      type        = "Federated"
      identifiers = [aws_iam_openid_connect_provider.github.arn]
    }
    actions = ["sts:AssumeRoleWithWebIdentity"]
    condition {
      test     = "StringLike"
      variable = "token.actions.githubusercontent.com:sub"
      values   = ["repo:greenticai/*:ref:refs/heads/*"]
    }
    condition {
      test     = "StringEquals"
      variable = "token.actions.githubusercontent.com:aud"
      values   = ["sts.amazonaws.com"]
    }
  }
}

resource "aws_iam_openid_connect_provider" "github" {
  url             = "https://token.actions.githubusercontent.com"
  client_id_list  = ["sts.amazonaws.com"]
  thumbprint_list = ["6938fd4d98bab03faadb97b34396831e3780aea1"]
}

resource "aws_iam_role" "ci" {
  name               = "${local.name}-ci"
  assume_role_policy = data.aws_iam_policy_document.ci_assume.json
}

resource "aws_iam_role_policy_attachment" "ci_ecr" {
  role       = aws_iam_role.ci.name
  policy_arn = "arn:${data.aws_partition.current.partition}:iam::aws:policy/AmazonEC2ContainerRegistryPowerUser"
}

resource "aws_iam_role_policy_attachment" "ci_apprunner" {
  role       = aws_iam_role.ci.name
  policy_arn = "arn:${data.aws_partition.current.partition}:iam::aws:policy/AWSAppRunnerFullAccess"
}

resource "aws_ecr_repository" "runner" {
  name                 = local.name
  image_tag_mutability = "MUTABLE"
  image_scanning_configuration {
    scan_on_push = true
  }
}

resource "aws_secretsmanager_secret" "runner" {
  for_each = toset(var.secrets)
  name     = "${local.name}/${each.value}"
}

resource "aws_cloudwatch_log_group" "runner" {
  name              = "/aws/apprunner/${local.name}"
  retention_in_days = 14
}

resource "aws_iam_role" "app_runner" {
  name               = "${local.name}-service"
  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect    = "Allow"
      Principal = { Service = "build.apprunner.amazonaws.com" }
      Action    = "sts:AssumeRole"
    }]
  })
}

resource "aws_iam_role_policy_attachment" "app_runner_ecr" {
  role       = aws_iam_role.app_runner.name
  policy_arn = "arn:${data.aws_partition.current.partition}:iam::aws:policy/service-role/AWSAppRunnerServicePolicyForECRAccess"
}

resource "aws_apprunner_service" "runner" {
  service_name = local.name
  source_configuration {
    authentication_configuration {
      access_role_arn = aws_iam_role.app_runner.arn
    }
    auto_deployments_enabled = true
    image_repository {
      image_identifier      = var.image
      image_repository_type = "ECR"
      image_configuration {
        port = "8080"
        runtime_environment_variables = {
          PORT                       = "8080"
          PACK_SOURCE                = "fs"
          PACK_INDEX_URL             = var.pack_index_url
          PACK_CACHE_DIR             = var.pack_cache_dir
          PACK_PUBLIC_KEY            = coalesce(var.pack_public_key, "")
          SECRETS_BACKEND            = var.secrets_backend
          TENANT_RESOLVER            = var.tenant_resolver
          PACK_REFRESH_INTERVAL      = var.pack_refresh_interval
          OTEL_EXPORTER_OTLP_ENDPOINT = var.telemetry_endpoint
          OTEL_SERVICE_NAME          = var.otel_service_name
        }
      }
    }
  }
  health_check_configuration {
    healthy_threshold   = 1
    interval            = 10
    path                = "/healthz"
    protocol            = "HTTP"
    timeout             = 5
    unhealthy_threshold = 3
  }
  observability_configuration {
    observability_configuration_arn = aws_apprunner_observability_configuration.otel.arn
    observability_enabled           = true
  }
}

resource "aws_apprunner_observability_configuration" "otel" {
  observability_configuration_name = "${local.name}-otel"
  trace_configuration {
    vendor = "AWSXRAY"
  }
}

