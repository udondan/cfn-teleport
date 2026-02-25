#!/bin/bash
# Helper function to check for drift by comparing deployed template against original
# Usage: check_drift <stack_name> <expect_changes> <template_file>
#   stack_name: Name of the CloudFormation stack to check
#   expect_changes: "yes" or "no" - whether changes are expected
#   template_file: Path to original template file to compare against

check_drift() {
  local stack_name="$1"
  local expect_changes="$2"  # "yes" or "no"
  local template_file="$3"   # original template file to compare against

  echo "Checking drift for stack: $stack_name (comparing against $template_file)"

  # Get current deployed template
  aws cloudformation get-template \
    --stack-name "$stack_name" \
    --query 'TemplateBody' \
    --output json | jq -r '.' > /tmp/deployed-${stack_name}.json

  # Get original template (normalize JSON)
  jq -S '.Resources | keys | sort' "$template_file" > /tmp/original-resources.json
  jq -S '.Resources | keys | sort' /tmp/deployed-${stack_name}.json > /tmp/deployed-resources.json

  # Compare resource lists
  if diff -q /tmp/original-resources.json /tmp/deployed-resources.json > /dev/null 2>&1; then
    echo "✓ No drift detected (resources match)"

    if [ "$expect_changes" = "yes" ]; then
      echo "✗ Expected changes but found none"
      echo "DEBUG: Original resources:"
      cat /tmp/original-resources.json
      echo "DEBUG: Deployed resources:"
      cat /tmp/deployed-resources.json
      return 1
    fi
    return 0
  else
    echo "✓ Drift detected (resources differ)"
    echo "Resources in original template:"
    cat /tmp/original-resources.json
    echo "Resources in deployed stack:"
    cat /tmp/deployed-resources.json

    if [ "$expect_changes" = "no" ]; then
      echo "✗ Expected no changes but found drift"
      return 1
    fi
    return 0
  fi
}

# If script is called directly (not sourced), call the function with passed arguments
if [ "${BASH_SOURCE[0]}" -eq "${0}" ]; then
  check_drift "$@"
fi
