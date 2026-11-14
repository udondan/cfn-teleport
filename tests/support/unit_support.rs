//! Unit test helpers: a fake CloudFormation API and scripted prompts.
//!
//! Included into the binary crate by `src/main.rs` when testing.
#![allow(dead_code)]

pub use crate::cfn_xml::{form_value, Reply};
use crate::Prompter;
use aws_credential_types::provider::error::CredentialsError;
use aws_credential_types::provider::future::ProvideCredentials as ProvideCredentialsFuture;
use aws_sdk_cloudformation as cloudformation;
use aws_sdk_cloudformation::config::{
    BehaviorVersion, Credentials, ProvideCredentials, Region, SharedCredentialsProvider,
};
use aws_smithy_runtime_api::client::http::{
    http_client_fn, HttpConnector, HttpConnectorFuture, SharedHttpConnector,
};
use aws_smithy_runtime_api::client::orchestrator::{HttpRequest, HttpResponse};
use aws_smithy_runtime_api::http::StatusCode;
use aws_smithy_types::body::SdkBody;
use aws_smithy_types::retry::RetryConfig;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::error::Error;
use std::sync::{Arc, Mutex};

/// Fake CloudFormation API that answers requests from a script, in order
#[derive(Debug, Clone, Default)]
pub struct FakeCfn {
    replies: Arc<Mutex<VecDeque<Reply>>>,
    requests: Arc<Mutex<Vec<(String, String)>>>,
}

impl FakeCfn {
    pub fn new(replies: Vec<Reply>) -> Self {
        Self {
            replies: Arc::new(Mutex::new(replies.into())),
            requests: Arc::default(),
        }
    }

    /// A client with static credentials that talks to this fake
    pub fn client(&self) -> cloudformation::Client {
        self.client_with_credentials(SharedCredentialsProvider::new(Credentials::new(
            "AKIDTEST", "secret", None, None, "test",
        )))
    }

    /// A client whose credentials provider fails like the default chain without credentials
    pub fn client_without_credentials(&self) -> cloudformation::Client {
        self.client_with_credentials(SharedCredentialsProvider::new(NoCredentials))
    }

    fn client_with_credentials(
        &self,
        credentials: SharedCredentialsProvider,
    ) -> cloudformation::Client {
        let fake = self.clone();
        let config = cloudformation::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("us-east-1"))
            .credentials_provider(credentials)
            .retry_config(RetryConfig::disabled())
            .http_client(http_client_fn(move |_, _| {
                SharedHttpConnector::new(fake.clone())
            }))
            .build();
        cloudformation::Client::from_conf(config)
    }

    /// Bodies of all requests made for the given action
    pub fn requests(&self, action: &str) -> Vec<String> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|(a, _)| a == action)
            .map(|(_, body)| body.clone())
            .collect()
    }

    /// Asserts that every scripted reply has been used
    pub fn assert_done(&self) {
        let left: Vec<String> = self
            .replies
            .lock()
            .unwrap()
            .iter()
            .map(|r| r.action.clone())
            .collect();
        assert!(left.is_empty(), "unused replies: {left:?}");
    }
}

impl HttpConnector for FakeCfn {
    fn call(&self, request: HttpRequest) -> HttpConnectorFuture {
        let body = String::from_utf8(request.body().bytes().unwrap().to_vec()).unwrap();
        let action = crate::cfn_xml::action_of(&body);
        self.requests
            .lock()
            .unwrap()
            .push((action.clone(), body.clone()));

        let reply = self.replies.lock().unwrap().pop_front();
        let reply = reply.unwrap_or_else(|| panic!("no reply left for {action}: {body}"));
        assert_eq!(reply.action, action, "unexpected request: {body}");

        let response = HttpResponse::new(
            StatusCode::try_from(reply.status).unwrap(),
            SdkBody::from(reply.body),
        );
        HttpConnectorFuture::ready(Ok(response))
    }
}

#[derive(Debug)]
struct NoCredentials;

impl ProvideCredentials for NoCredentials {
    fn provide_credentials<'a>(&'a self) -> ProvideCredentialsFuture<'a>
    where
        Self: 'a,
    {
        ProvideCredentialsFuture::ready(Err(CredentialsError::not_loaded(
            "no providers in chain provided credentials",
        )))
    }
}

/// One scripted answer to a prompt
#[derive(Debug, Clone)]
pub enum Answer {
    Select(Option<usize>),
    MultiSelect(Option<Vec<usize>>),
    /// None means: accept the default
    Input(Option<String>),
    Confirm(Option<bool>),
}

/// Prompter that returns scripted answers in order
#[derive(Debug, Default)]
pub struct ScriptedPrompter {
    answers: RefCell<VecDeque<Answer>>,
    pub prompts: RefCell<Vec<String>>,
}

impl ScriptedPrompter {
    pub fn new(answers: Vec<Answer>) -> Self {
        Self {
            answers: RefCell::new(answers.into()),
            prompts: RefCell::default(),
        }
    }

    fn next(&self, prompt: &str) -> Answer {
        self.prompts.borrow_mut().push(prompt.to_string());
        let answer = self.answers.borrow_mut().pop_front();
        answer.unwrap_or_else(|| panic!("no answer left for prompt: {prompt}"))
    }
}

impl Prompter for ScriptedPrompter {
    fn select(&self, prompt: &str, _items: &[&str]) -> Result<Option<usize>, Box<dyn Error>> {
        match self.next(prompt) {
            Answer::Select(answer) => Ok(answer),
            other => Err(format!("expected select answer, got {other:?}").into()),
        }
    }

    fn multi_select(
        &self,
        prompt: &str,
        _items: &[String],
    ) -> Result<Option<Vec<usize>>, Box<dyn Error>> {
        match self.next(prompt) {
            Answer::MultiSelect(answer) => Ok(answer),
            other => Err(format!("expected multi select answer, got {other:?}").into()),
        }
    }

    fn input(&self, prompt: &str, default: &str) -> Result<String, Box<dyn Error>> {
        match self.next(prompt) {
            Answer::Input(answer) => Ok(answer.unwrap_or_else(|| default.to_string())),
            other => Err(format!("expected input answer, got {other:?}").into()),
        }
    }

    fn confirm(&self, prompt: &str) -> Result<Option<bool>, Box<dyn Error>> {
        match self.next(prompt) {
            Answer::Confirm(answer) => Ok(answer),
            other => Err(format!("expected confirm answer, got {other:?}").into()),
        }
    }
}
