# Test helper functions for cfn-teleport integration tests (PowerShell)
# Source this file in PowerShell test scripts: . .\test\cloudformation\test-helpers.ps1

# Wait for a CloudFormation stack to reach a stable (non-IN_PROGRESS) state
# Usage: Wait-ForStackStable "StackName"
function Wait-ForStackStable {
  param (
    [string]$StackName
  )

  Write-Host "⏳ Waiting for stack $StackName to reach stable state..."
  while ($true) {
    try {
      $status = aws cloudformation describe-stacks --stack-name $StackName --query 'Stacks[0].StackStatus' --output text 2>$null
      if ($LASTEXITCODE -ne 0) {
        Write-Error "✗ Stack $StackName not found"
        return $false
      }

      # Check if status ends with _IN_PROGRESS
      if ($status -like "*_IN_PROGRESS") {
        Write-Host "  Stack status: $status (waiting...)"
        Start-Sleep -Seconds 2
      } else {
        Write-Host "✓ Stack $StackName reached stable state: $status"
        break
      }
    } catch {
      Write-Error "✗ Failed to get status for stack $StackName"
      return $false
    }
  }
  return $true
}
