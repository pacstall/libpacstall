//! Errors used by the store

use std::fmt;

use error_stack::Context;

/// Given store query yielded no results
#[derive(Debug, Clone)]
pub struct NoQueryMatchError;

impl fmt::Display for NoQueryMatchError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("query yielded no results")
    }
}

impl Context for NoQueryMatchError {}

/// Store mutation failed
#[derive(Debug, Clone)]
pub struct EntityMutationError;

impl fmt::Display for EntityMutationError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("store mutation failed")
    }
}

impl Context for EntityMutationError {}

/// Error representation of a failed IO operation.
#[derive(Debug, Clone)]
pub struct IOError;

impl fmt::Display for IOError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("failed to do IO operation")
    }
}

impl Context for IOError {}

/// Generic store error representation.
#[derive(Debug, Clone)]
pub struct StoreError;

impl fmt::Display for StoreError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("store operation failed")
    }
}

impl Context for StoreError {}

/// Error representation for entities that are not found.
#[derive(Debug, Clone)]
pub struct EntityNotFoundError;

impl fmt::Display for EntityNotFoundError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result { fmt.write_str("entity not found") }
}

impl Context for EntityNotFoundError {}

/// Error representation for entities that already exist, but shouldn't.
#[derive(Debug, Clone)]
pub struct EntityAlreadyExistsError;

impl fmt::Display for EntityAlreadyExistsError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("entity already exists")
    }
}

impl Context for EntityAlreadyExistsError {}
