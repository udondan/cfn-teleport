//! End-to-end tests of the `cfn-teleport` binary.
//!
//! The binary talks to a fake CloudFormation API on localhost. Interactive runs use a
//! pseudo terminal, so the real prompts and the terminal spinner are exercised.
#![cfg(unix)]

#[path = "support/cfn_xml.rs"]
mod cfn_xml;

use cfn_xml::{action_of, Reply};
use rexpect::process::WaitStatus;
use rexpect::session::spawn_command;
use std::collections::VecDeque;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

/// Fake CloudFormation API that answers requests with scripted replies, in order
struct FakeServer {
    url: String,
    replies: Arc<Mutex<VecDeque<Reply>>>,
    actions: Arc<Mutex<Vec<String>>>,
}

impl FakeServer {
    fn start(replies: Vec<Reply>) -> Self {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.server_addr().to_ip().unwrap());
        let replies = Arc::new(Mutex::new(VecDeque::from(replies)));
        let actions = Arc::new(Mutex::new(Vec::new()));

        let (queue, seen) = (replies.clone(), actions.clone());
        thread::spawn(move || {
            for mut request in server.incoming_requests() {
                let mut body = String::new();
                request.as_reader().read_to_string(&mut body).unwrap();
                let action = action_of(&body);
                seen.lock().unwrap().push(action.clone());

                // An unexpected request gets an error, which makes the binary fail
                let reply = queue
                    .lock()
                    .unwrap()
                    .pop_front()
                    .filter(|r| r.action == action)
                    .unwrap_or_else(|| Reply::error(&action, "Unexpected", &action));
                let response = tiny_http::Response::from_string(reply.body)
                    .with_status_code(reply.status)
                    .with_header(
                        tiny_http::Header::from_bytes("Content-Type", "text/xml").unwrap(),
                    );
                request.respond(response).unwrap();
            }
        });

        Self {
            url,
            replies,
            actions,
        }
    }

    fn command(&self, args: &[&str], work_dir: &Path) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_cfn-teleport"));
        command
            .args(args)
            .current_dir(work_dir)
            .env("AWS_ENDPOINT_URL", &self.url)
            .env("AWS_ACCESS_KEY_ID", "test")
            .env("AWS_SECRET_ACCESS_KEY", "test")
            .env("AWS_REGION", "us-east-1")
            .env("AWS_CONFIG_FILE", "/dev/null")
            .env("AWS_SHARED_CREDENTIALS_FILE", "/dev/null")
            .env("AWS_EC2_METADATA_DISABLED", "true")
            .env("TERM", "xterm-256color")
            .env_remove("AWS_PROFILE")
            .env_remove("AWS_SESSION_TOKEN");
        command
    }

    /// Asserts that every scripted reply has been used
    fn assert_done(&self) {
        let left: Vec<String> = self
            .replies
            .lock()
            .unwrap()
            .iter()
            .map(|r| r.action.clone())
            .collect();
        assert!(
            left.is_empty(),
            "unused replies {left:?}, requests: {:?}",
            self.actions.lock().unwrap()
        );
    }
}

fn template() -> String {
    r#"{"Resources": {"Bucket": {"Type": "AWS::S3::Bucket"}, "Queue": {"Type": "AWS::SQS::Queue"}}}"#
        .to_string()
}

/// Replies until the stack refactor is created
fn rename_setup() -> Vec<Reply> {
    vec![
        Reply::list_stacks(
            &[("Target", "CREATE_COMPLETE"), ("Source", "UPDATE_COMPLETE")],
            None,
        ),
        Reply::list_stack_resources(&[
            ("Queue", "my-queue", "AWS::SQS::Queue"),
            ("Bucket", "my-bucket", "AWS::S3::Bucket"),
        ]),
        Reply::get_template(&template()),
        Reply::validate_template(),
        Reply::create_stack_refactor(Some("refactor-1")),
    ]
}

const TIMEOUT_MS: Option<u64> = Some(30_000);

#[test]
fn test_invalid_arguments_exit_with_error() {
    let dir = tempfile::tempdir().unwrap();
    let server = FakeServer::start(vec![]);

    let output = server
        .command(&["--out-dir", "out"], dir.path())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Cannot use --out-dir without --export"),
        "{stderr}"
    );
}

#[test]
fn test_export_without_terminal() {
    let dir = tempfile::tempdir().unwrap();
    let server = FakeServer::start(rename_setup()[..3].to_vec());

    let output = server
        .command(
            &[
                "-s",
                "Source",
                "-t",
                "Source",
                "-r",
                "Bucket:NewBucket",
                "--yes",
                "--export",
            ],
            dir.path(),
        )
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{stdout}");
    assert!(
        stdout.contains("Templates exported successfully"),
        "{stdout}"
    );
    server.assert_done();
}

#[test]
fn test_interactive_rename() {
    let dir = tempfile::tempdir().unwrap();
    let mut replies = rename_setup();
    replies.extend([
        Reply::describe_stack_refactor(Some("CREATE_COMPLETE"), None, None),
        Reply::execute_stack_refactor(),
        Reply::describe_stack_refactor(None, Some("EXECUTE_COMPLETE"), None),
    ]);
    let server = FakeServer::start(replies);
    let mut session = spawn_command(server.command(&[], dir.path()), TIMEOUT_MS).unwrap();

    // Stacks are sorted, so "Source" comes first
    session.exp_string("Select source stack").unwrap();
    session.send_line("").unwrap();
    session.exp_string("Select target stack").unwrap();
    session.send_line("").unwrap();
    // Resources are sorted by type, so "Bucket" comes first. Space selects it.
    session.exp_string("Select resources to copy").unwrap();
    session.send(" ").unwrap();
    session.send_line("").unwrap();
    session
        .exp_string("new logical ID for resource 'Bucket'")
        .unwrap();
    session.send_line("NewBucket").unwrap();
    session.exp_string("Please confirm your selection").unwrap();
    session.send_line("y").unwrap();
    session
        .exp_string("Renamed 1 resource in stack Source")
        .unwrap();
    session.exp_eof().unwrap();

    let status = session.process().wait().unwrap();
    assert!(matches!(status, WaitStatus::Exited(_, 0)), "{status:?}");
    server.assert_done();
}

#[test]
fn test_failed_refactor_in_terminal() {
    let dir = tempfile::tempdir().unwrap();
    let mut replies = rename_setup();
    replies.push(Reply::describe_stack_refactor(
        Some("CREATE_FAILED"),
        None,
        Some("boom"),
    ));
    let server = FakeServer::start(replies);
    let command = server.command(
        &[
            "-s",
            "Source",
            "-t",
            "Source",
            "-r",
            "Bucket:NewBucket",
            "--yes",
        ],
        dir.path(),
    );
    let mut session = spawn_command(command, TIMEOUT_MS).unwrap();

    session
        .exp_string("Stack refactor validation failed: boom")
        .unwrap();
    session.exp_eof().unwrap();

    let status = session.process().wait().unwrap();
    assert!(matches!(status, WaitStatus::Exited(_, 1)), "{status:?}");
    server.assert_done();
}
