#!/bin/bash
# Test helper functions for cfn-teleport integration tests
# Source this file in test scripts to use these functions

# Wait for a CloudFormation stack to reach a stable (non-IN_PROGRESS) state
# Usage: wait_for_stack_stable "StackName"
wait_for_stack_stable() {
  local stack_name="$1"
  echo "⏳ Waiting for stack $stack_name to reach stable state..."
  while true; do
    local status=$(aws cloudformation describe-stacks --stack-name "$stack_name" --query 'Stacks[0].StackStatus' --output text 2>/dev/null || echo "NOT_FOUND")

    if [[ "$status" == "NOT_FOUND" ]]; then
      echo "✗ Stack $stack_name not found"
      return 1
    fi

    # Check if status ends with _IN_PROGRESS
    if [[ "$status" == *_IN_PROGRESS ]]; then
      echo "  Stack status: $status (waiting...)"
      sleep 2
    else
      echo "✓ Stack $stack_name reached stable state: $status"
      break
    fi
  done
}

# Run cfn-teleport with visible output and automatic stack state waiting
# This wrapper ensures stacks reach stable state before returning
# Usage: run_cfn_teleport --source Stack1 --target Stack2 --yes --resource Foo
run_cfn_teleport() {
  echo ""
  echo "═══════════════════════════════════════════════════════════════════════════════"
  echo "▶ Running: cfn-teleport $*"
  echo "───────────────────────────────────────────────────────────────────────────────"
  cfn-teleport "$@" 2>&1
  local exit_code=$?
  echo "───────────────────────────────────────────────────────────────────────────────"
  echo "▶ Exit code: $exit_code"
  echo "═══════════════════════════════════════════════════════════════════════════════"
  echo ""

  # Wait for affected stacks to reach stable state after successful operations
  if [ $exit_code -eq 0 ]; then
    # Parse --source and --target from arguments
    local source_stack=""
    local target_stack=""
    local prev_arg=""
    for arg in "$@"; do
      if [ "$prev_arg" = "--source" ]; then
        source_stack="$arg"
      elif [ "$prev_arg" = "--target" ]; then
        target_stack="$arg"
      fi
      prev_arg="$arg"
    done

    # Wait for source stack if specified
    if [ -n "$source_stack" ]; then
      wait_for_stack_stable "$source_stack" || return 1
    fi

    # Wait for target stack if specified and different from source
    if [ -n "$target_stack" ] && [ "$target_stack" != "$source_stack" ]; then
      wait_for_stack_stable "$target_stack" || return 1
    fi
  fi

  return $exit_code
}

# Check drift using Makefile target
# Usage: check_drift "StackName" "yes|no" "TemplatePath"
check_drift() {
  local stack="$1"
  local expect="$2"
  local template="$3"
  cd test/cloudformation
  make check-drift STACK="$stack" EXPECT="$expect" TEMPLATE="$template"
  local result=$?
  cd ../..
  return $result
}

# Verify stack resources match original template
# Usage: verify_stack "StackName" "TemplatePath"
verify_stack() {
  local stack_name="$1"
  local template_file="$2"

  echo "Checking $stack_name..."

  # Get deployed template - AWS returns TemplateBody as a JSON string or object depending on format
  # We need to extract it first with jq -r, then parse it
  if ! aws cloudformation get-template \
    --stack-name "$stack_name" \
    --output json > /tmp/aws-response-${stack_name}.json 2>/tmp/aws-error-${stack_name}.txt; then
    echo "❌ Failed to get deployed template"
    cat /tmp/aws-error-${stack_name}.txt
    return 1
  fi

  # Extract TemplateBody and parse resources
  if ! jq -r '.TemplateBody' /tmp/aws-response-${stack_name}.json > /tmp/deployed-raw-${stack_name}.txt 2>/tmp/jq1-error-${stack_name}.txt; then
    echo "❌ Failed to extract TemplateBody"
    cat /tmp/jq1-error-${stack_name}.txt
    return 1
  fi

  # Try parsing as JSON first, fall back to YAML
  if jq -cS '.Resources | keys | sort' /tmp/deployed-raw-${stack_name}.txt > /tmp/deployed-${stack_name}.json 2>/dev/null; then
    : # Successfully parsed as JSON
  else
    # Must be YAML, convert it
    if ! command -v yq &> /dev/null; then
      echo "❌ yq not found. Install with: brew install yq"
      return 1
    fi
    if ! yq -o=json '.Resources | keys | sort' /tmp/deployed-raw-${stack_name}.txt > /tmp/deployed-${stack_name}.json 2>/tmp/yq-deployed-error-${stack_name}.txt; then
      echo "❌ Failed to parse deployed template as YAML"
      cat /tmp/yq-deployed-error-${stack_name}.txt
      return 1
    fi
  fi

  # Get original template resources (handle both JSON and YAML)
  if echo "$template_file" | grep -q '\.yaml$'; then
    # YAML template - use yq to convert to JSON, then extract resources
    if ! yq -o=json '.Resources | keys | sort' "$template_file" > /tmp/original-${stack_name}.json 2>/tmp/yq-error-${stack_name}.txt; then
      echo "❌ Failed to parse YAML template"
      cat /tmp/yq-error-${stack_name}.txt
      return 1
    fi
  else
    # JSON template
    if ! jq -cS '.Resources | keys | sort' "$template_file" > /tmp/original-${stack_name}.json 2>/tmp/jq3-error-${stack_name}.txt; then
      echo "❌ Failed to parse JSON template"
      cat /tmp/jq3-error-${stack_name}.txt
      return 1
    fi
  fi

  if diff -q /tmp/original-${stack_name}.json /tmp/deployed-${stack_name}.json > /dev/null 2>&1; then
    echo "✅ $stack_name resources match original template"
    return 0
  else
    echo "❌ $stack_name resources differ from original template"
    echo "Expected resources:"
    cat /tmp/original-${stack_name}.json
    echo "Actual resources:"
    cat /tmp/deployed-${stack_name}.json
    return 1
  fi
}
