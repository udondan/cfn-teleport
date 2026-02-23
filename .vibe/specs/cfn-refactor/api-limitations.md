# CloudFormation Stack Refactoring API Limitations

*Discovered during T008 testing on 2026-02-21*

## Overview

The CloudFormation Stack Refactoring API has **MORE restrictive resource type support** than the import/export API. Our initial assumption that refactoring would support the same 1304 resource types as import was incorrect.

## Known Unsupported Resource Types

### AWS::EC2::KeyPair
**Reason**: Does not allow tag updates after creation

**Error message from API**:
```
Stack Refactor does not support AWS::EC2::KeyPair because the resource type 
does not allow tag updates after creation.
```

**Explanation**: CloudFormation uses tags internally to track resource ownership during stack refactoring. Resource types that don't support tag updates cannot be refactored between stacks.

---

## Property Mismatch Validation

### Issue
The Stack Refactoring API performs strict validation that resource properties in the source template **exactly match** the destination template. This can cause failures when:

1. **Stack-specific values differ**: Resource names, stack references, etc.
2. **Conditional properties**: Properties that exist in one template but not the other
3. **Generated values**: Auto-generated or stack-specific attributes

**Example error from testing**:
```
Resource SecurityGroupDD263621 in stack arn:aws:cloudformation:us-east-1:031700846815:stack/CfnTeleportTest1/... 
does not match the destination resource's properties.
```

**Tested resources with issues**:
- `AWS::EC2::SecurityGroup` - Property mismatch (needs investigation)
- `AWS::IAM::Role` - Property mismatch (needs investigation)

**Successfully moved**:
- ✅ `AWS::S3::Bucket` - Works reliably, multiple buckets tested
- ✅ `AWS::DynamoDB::Table` - Works reliably

**Next steps**:
- Investigate which specific properties are mismatching for IAM/EC2
- Determine if template preparation logic needs updates
- Test more resource types systematically (Lambda, SQS, etc.)

## Impact on cfn-teleport

### Current Behavior
- Tool attempts refactoring operation
- AWS API validates and returns clear error message
- Operation fails atomically (no stacks modified)
- Error message is propagated to user

### Implications
1. **Not all resources can be moved** - Users need to understand this limitation
2. **Import/export flow was more permissive** - But at the cost of potential corruption
3. **Trade-off**: Atomic validation vs. broader resource support

## Questions to Resolve

1. **Should we maintain a known-unsupported list?**
   - **Pro**: Pre-validate and provide better error messages earlier
   - **Con**: Hard to maintain, API error messages are already clear
   - **Recommendation**: Rely on API error messages (they're descriptive)

2. **Should we offer a fallback to import/export for unsupported types?**
   - **Pro**: Broader resource type support
   - **Con**: Re-introduces the corruption risk we're trying to fix
   - **Con**: Significantly more complex code
   - **Recommendation**: NO - fail fast with clear error, document limitations

3. **How do we discover all unsupported types?**
   - **Option 1**: Wait for users to report them (current approach)
   - **Option 2**: Test all 1304 resource types systematically
   - **Option 3**: Contact AWS support for official list
   - **Recommendation**: Document as discovered, update this file

## Documentation Needs

- Update README.md with resource type limitations
- Add section to error handling docs
- Update spec.md with refined resource type support section
- Consider adding a `--dry-run` flag that validates without executing

## Related Tasks

- T008: Test core refactoring function
- T008a: Handle unsupported resource types (error messages)
- T018: Handle common error scenarios
- T020: Update CLI help and documentation
