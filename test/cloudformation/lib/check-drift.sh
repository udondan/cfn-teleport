#!/usr/bin/env bash
set -euo pipefail

# check_drift: Detect if CloudFormation stack has drifted from template
# Equivalent to: npx cdk diff --fail
#
# Usage:
#   check_drift <stack-name> <template-file>
#
# Returns:
#   - Exit 0: No drift detected (equivalent to cdk diff --fail success)
#   - Exit 1: Drift detected (equivalent to cdk diff showing changes)
#   - Exit 2: Error (stack doesn't exist, invalid template, etc.)

check_drift() {
  local stack_name=$1
  local template_file=$2
  local change_set_name="drift-check-$(date +%s)"
  
  echo "Checking drift for stack: $stack_name"
  
  # Check if stack exists
  if ! aws cloudformation describe-stacks --stack-name "$stack_name" &>/dev/null; then
    echo "ERROR: Stack $stack_name does not exist"
    return 2
  fi
  
  # Try to create change set with template file
  local output
  output=$(aws cloudformation create-change-set \
    --stack-name "$stack_name" \
    --change-set-name "$change_set_name" \
    --template-body "file://$template_file" \
    --change-set-type UPDATE \
    --capabilities CAPABILITY_IAM \
    2>&1 || true)
  
  # Check for "no changes" message
  if echo "$output" | grep -qi "didn't contain changes\|No updates"; then
    echo "✓ No drift detected (no changes needed)"
    return 0
  elif echo "$output" | grep -qi "arn:aws:cloudformation"; then
    # Change set created successfully - means there are changes
    echo "✗ Drift detected - changes found"
    
    # Wait for change set to be ready
    aws cloudformation wait change-set-create-complete \
      --stack-name "$stack_name" \
      --change-set-name "$change_set_name" 2>/dev/null || true
    
    # Describe changes
    local changes
    changes=$(aws cloudformation describe-change-set \
      --stack-name "$stack_name" \
      --change-set-name "$change_set_name" \
      --query 'Changes' \
      --output json 2>/dev/null || echo "[]")
    
    if [ "$changes" != "[]" ] && [ "$changes" != "null" ]; then
      echo "$changes" | jq -r '.[] | "  - \(.Type): \(.ResourceChange.LogicalResourceId) (\(.ResourceChange.Action))"' 2>/dev/null || true
    fi
    
    # Cleanup change set
    aws cloudformation delete-change-set \
      --stack-name "$stack_name" \
      --change-set-name "$change_set_name" 2>/dev/null || true
    
    return 1
  else
    echo "ERROR: Unexpected error creating change set"
    echo "$output"
    return 2
  fi
}

# If sourced, make function available; if executed, run with arguments
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  check_drift "$@"
fi
