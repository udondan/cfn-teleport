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
