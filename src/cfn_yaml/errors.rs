// Copyright 2020-2022 Amazon Web Services, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
//
// Error types adapted from AWS CloudFormation Guard

use std::fmt;

pub(crate) type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub(crate) enum Error {
    ParseError(String),
    InternalError(InternalError),
}

#[derive(Debug, Clone)]
pub(crate) enum InternalError {
    InvalidKeyType(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ParseError(msg) => write!(f, "Parse error: {}", msg),
            Error::InternalError(err) => write!(f, "Internal error: {:?}", err),
        }
    }
}

impl std::error::Error for Error {}

impl From<InternalError> for Error {
    fn from(err: InternalError) -> Self {
        Error::InternalError(err)
    }
}
