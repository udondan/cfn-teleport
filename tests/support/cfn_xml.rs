//! Canned CloudFormation API responses (AWS Query protocol, XML).
//!
//! Shared by the unit tests in `src/` and the end-to-end tests in `tests/`.
#![allow(dead_code)]

/// One scripted reply of the fake CloudFormation API
#[derive(Debug, Clone)]
pub struct Reply {
    /// API action the request must have, e.g. `ListStacks`
    pub action: String,
    pub status: u16,
    pub body: String,
}

impl Reply {
    /// Successful response. `result` is the XML inside `<{Action}Result>`.
    pub fn ok(action: &str, result: &str) -> Self {
        Self {
            action: action.to_string(),
            status: 200,
            body: format!(
                "<{action}Response xmlns=\"http://cloudformation.amazonaws.com/doc/2010-05-15/\">\
                 <{action}Result>{result}</{action}Result>\
                 <ResponseMetadata><RequestId>req-1</RequestId></ResponseMetadata>\
                 </{action}Response>"
            ),
        }
    }

    /// Error response with code and message
    pub fn error(action: &str, code: &str, message: &str) -> Self {
        Self {
            action: action.to_string(),
            status: 400,
            body: format!(
                "<ErrorResponse xmlns=\"http://cloudformation.amazonaws.com/doc/2010-05-15/\">\
                 <Error><Type>Sender</Type><Code>{}</Code><Message>{}</Message></Error>\
                 <RequestId>req-1</RequestId></ErrorResponse>",
                escape(code),
                escape(message)
            ),
        }
    }

    /// Error response without any details
    pub fn bare_error(action: &str) -> Self {
        Self {
            action: action.to_string(),
            status: 400,
            body: "<ErrorResponse><Error></Error></ErrorResponse>".to_string(),
        }
    }

    pub fn list_stacks(stacks: &[(&str, &str)], next_token: Option<&str>) -> Self {
        let members: String = stacks
            .iter()
            .map(|(name, status)| {
                format!(
                    "<member><StackName>{name}</StackName><StackId>id-{name}</StackId>\
                     <StackStatus>{status}</StackStatus>\
                     <CreationTime>2024-01-01T00:00:00Z</CreationTime></member>"
                )
            })
            .collect();
        let token = next_token
            .map(|t| format!("<NextToken>{t}</NextToken>"))
            .unwrap_or_default();
        Self::ok(
            "ListStacks",
            &format!("<StackSummaries>{members}</StackSummaries>{token}"),
        )
    }

    /// `resources` are (logical id, physical id, type)
    pub fn list_stack_resources(resources: &[(&str, &str, &str)]) -> Self {
        let members: String = resources
            .iter()
            .map(|(logical, physical, typ)| {
                format!(
                    "<member><LogicalResourceId>{logical}</LogicalResourceId>\
                     <PhysicalResourceId>{physical}</PhysicalResourceId>\
                     <ResourceType>{typ}</ResourceType>\
                     <LastUpdatedTimestamp>2024-01-01T00:00:00Z</LastUpdatedTimestamp>\
                     <ResourceStatus>CREATE_COMPLETE</ResourceStatus></member>"
                )
            })
            .collect();
        Self::ok(
            "ListStackResources",
            &format!("<StackResourceSummaries>{members}</StackResourceSummaries>"),
        )
    }

    pub fn get_template(body: &str) -> Self {
        Self::ok(
            "GetTemplate",
            &format!("<TemplateBody>{}</TemplateBody>", escape(body)),
        )
    }

    pub fn validate_template() -> Self {
        Self::ok("ValidateTemplate", "")
    }

    pub fn update_stack() -> Self {
        Self::ok("UpdateStack", "<StackId>id</StackId>")
    }

    pub fn describe_stacks(status: &str) -> Self {
        Self::ok(
            "DescribeStacks",
            &format!(
                "<Stacks><member><StackName>s</StackName><StackStatus>{status}</StackStatus>\
                 <CreationTime>2024-01-01T00:00:00Z</CreationTime></member></Stacks>"
            ),
        )
    }

    /// `identifiers` are (resource type, logical ids, identifier property)
    pub fn get_template_summary(identifiers: &[(&str, &[&str], &str)]) -> Self {
        let members: String = identifiers
            .iter()
            .map(|(typ, logical_ids, identifier)| {
                let ids: String = logical_ids
                    .iter()
                    .map(|id| format!("<member>{id}</member>"))
                    .collect();
                format!(
                    "<member><ResourceType>{typ}</ResourceType>\
                     <LogicalResourceIds>{ids}</LogicalResourceIds>\
                     <ResourceIdentifiers><member>{identifier}</member></ResourceIdentifiers></member>"
                )
            })
            .collect();
        Self::ok(
            "GetTemplateSummary",
            &format!("<ResourceIdentifierSummaries>{members}</ResourceIdentifierSummaries>"),
        )
    }

    pub fn create_change_set() -> Self {
        Self::ok("CreateChangeSet", "<Id>cs-id</Id><StackId>id</StackId>")
    }

    pub fn describe_change_set(status: &str, reason: Option<&str>) -> Self {
        let reason = reason
            .map(|r| format!("<StatusReason>{}</StatusReason>", escape(r)))
            .unwrap_or_default();
        Self::ok(
            "DescribeChangeSet",
            &format!("<Status>{status}</Status>{reason}"),
        )
    }

    pub fn execute_change_set() -> Self {
        Self::ok("ExecuteChangeSet", "")
    }

    pub fn create_stack_refactor(id: Option<&str>) -> Self {
        let id = id
            .map(|id| format!("<StackRefactorId>{id}</StackRefactorId>"))
            .unwrap_or_default();
        Self::ok("CreateStackRefactor", &id)
    }

    /// `status` is the validation status, `execution_status` the execution status
    pub fn describe_stack_refactor(
        status: Option<&str>,
        execution_status: Option<&str>,
        reason: Option<&str>,
    ) -> Self {
        let mut result = String::from("<StackRefactorId>refactor-1</StackRefactorId>");
        if let Some(status) = status {
            result.push_str(&format!("<Status>{status}</Status>"));
        }
        if let Some(execution_status) = execution_status {
            result.push_str(&format!(
                "<ExecutionStatus>{execution_status}</ExecutionStatus>"
            ));
        }
        if let Some(reason) = reason {
            result.push_str(&format!("<StatusReason>{}</StatusReason>", escape(reason)));
        }
        Self::ok("DescribeStackRefactor", &result)
    }

    pub fn execute_stack_refactor() -> Self {
        Self::ok("ExecuteStackRefactor", "")
    }
}

/// Escapes text for use inside XML elements
pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Returns the action of a form encoded query request body
pub fn action_of(body: &str) -> String {
    form_value(body, "Action").unwrap_or_default()
}

/// Returns a decoded value of a form encoded request body
pub fn form_value(body: &str, key: &str) -> Option<String> {
    body.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        (percent_decode(k) == key).then(|| percent_decode(v))
    })
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap();
                out.push(u8::from_str_radix(hex, 16).unwrap());
                i += 2;
            }
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8(out).unwrap()
}
