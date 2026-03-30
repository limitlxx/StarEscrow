//! Typed error system for the StarEscrow CLI.
//!
//! This module defines `CliError`, a comprehensive enum covering all CLI-specific
//! error cases. For unexpected system failures or third-party errors not covered
//! by `CliError`, the `anyhow` crate is still used with `?` operator for ergonomic
//! error propagation.
//!
//! All `CliError` variants include clear, user-friendly error messages suitable
//! for display to end users.

use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    /// Invalid Stellar secret key format or derivation failure.
    ///
    /// Stellar secret keys must be valid strkeys starting with 'S'
    /// and encoding 32 bytes of ed25519 seed material.
    #[error("Invalid Stellar secret key: {0}\nExpected format: S...")]
    InvalidStellarSecretKey(String),

    /// Invalid deadline specification.
    ///
    /// Deadlines must be either:
    ///   - A Unix timestamp (integer seconds since epoch), or
    ///   - An ISO 8601 datetime string (e.g., "2026-12-31T23:59:59Z")
    #[error("Invalid deadline format: {0}\nProvide either a Unix timestamp or ISO 8601 datetime (e.g., 2026-12-31T23:59:59Z)")]
    InvalidDeadline(String),

    /// WASM file not found at the specified path.
    #[error("WASM file not found: {0}")]
    WasmFileNotFound(PathBuf),

    /// Failed to read a file due to I/O error.
    #[error("Failed to read file {path}: {details}")]
    FileReadError {
        path: PathBuf,
        details: String,
    },

    /// Invalid Stellar contract ID format.
    ///
    /// Contract IDs must be valid strkeys starting with 'C'
    /// and encoding 32 bytes of contract address material.
    #[error("Invalid Stellar contract ID: {0}\nExpected format: C...")]
    InvalidContractId(String),

    /// Invalid input provided to a command.
    ///
    /// Covers generic input validation failures not covered by more specific variants.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// RPC (Soroban JSON-RPC) operation failed.
    ///
    /// Includes network errors, RPC-level errors, and invalid RPC responses.
    #[error("RPC error: {0}")]
    RpcError(String),

    /// Configuration file error.
    ///
    /// Covers missing, malformed, or unreadable configuration files.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// JSON serialization or deserialization error.
    ///
    /// Occurs when encoding/decoding contract arguments or responses.
    #[error("JSON processing error: {0}")]
    SerializationError(String),

    /// Missing or invalid command-line argument.
    ///
    /// Argument validation that goes beyond what clap easily handles
    /// (e.g., semantic validation of paired arguments).
    #[error("Missing or invalid argument: {0}")]
    InvalidArgument(String),

    /// Cryptographic operation failure.
    ///
    /// Includes keypair generation, signature creation, or verification failures.
    #[error("Cryptographic operation failed: {0}")]
    CryptoError(String),

    /// User confirmation or interaction denied.
    ///
    /// Raised when a required user action (e.g., prompt confirmation) fails or is denied.
    #[error("Operation cancelled: {0}")]
    OperationCancelled(String),

    /// Contract execution failure during simulation or submission.
    ///
    /// The contract invocation was syntactically valid but failed during execution.
    #[error("Contract execution failed: {0}")]
    ContractExecutionError(String),

    /// Invalid Stellar address format.
    ///
    /// Stellar addresses must be valid strkeys starting with 'G'
    /// and encoding 32 bytes of public key material.
    #[error("Invalid Stellar address: {0}\nExpected format: G...")]
    InvalidStellarAddress(String),

    /// Command execution failed.
    ///
    /// A general-purpose error for when a subcommand fails for reasons
    /// not covered by more specific variants.
    #[error("Command failed: {0}")]
    CommandError(String),
}

impl CliError {
    /// Create an `InvalidStellarSecretKey` error with context.
    pub fn invalid_secret_key(source: &str) -> Self {
        Self::InvalidStellarSecretKey(source.to_string())
    }

    /// Create an `InvalidDeadline` error with context.
    pub fn invalid_deadline(input: &str) -> Self {
        Self::InvalidDeadline(input.to_string())
    }

    /// Create a `WasmFileNotFound` error.
    pub fn wasm_not_found(path: impl AsRef<Path>) -> Self {
        Self::WasmFileNotFound(path.as_ref().to_path_buf())
    }

    /// Create a `FileReadError` with path and I/O error details.
    pub fn file_read_error(path: impl AsRef<Path>, error: impl std::error::Error) -> Self {
        Self::FileReadError {
            path: path.as_ref().to_path_buf(),
            details: error.to_string(),
        }
    }

    /// Create an `InvalidContractId` error.
    pub fn invalid_contract_id(id: &str) -> Self {
        Self::InvalidContractId(id.to_string())
    }

    /// Create an `InvalidInput` error.
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::InvalidInput(msg.into())
    }

    /// Create an `RpcError` with context.
    pub fn rpc_error(msg: impl Into<String>) -> Self {
        Self::RpcError(msg.into())
    }

    /// Create a `ConfigError` with context.
    pub fn config_error(msg: impl Into<String>) -> Self {
        Self::ConfigError(msg.into())
    }

    /// Create a `SerializationError` with context.
    pub fn serialization_error(msg: impl Into<String>) -> Self {
        Self::SerializationError(msg.into())
    }

    /// Create an `InvalidArgument` error.
    pub fn invalid_argument(msg: impl Into<String>) -> Self {
        Self::InvalidArgument(msg.into())
    }

    /// Create a `CryptoError` with context.
    pub fn crypto_error(msg: impl Into<String>) -> Self {
        Self::CryptoError(msg.into())
    }

    /// Create an `OperationCancelled` error.
    pub fn operation_cancelled(reason: impl Into<String>) -> Self {
        Self::OperationCancelled(reason.into())
    }

    /// Create a `ContractExecutionError` with context.
    pub fn contract_execution_error(msg: impl Into<String>) -> Self {
        Self::ContractExecutionError(msg.into())
    }

    /// Create an `InvalidStellarAddress` error.
    pub fn invalid_stellar_address(addr: &str) -> Self {
        Self::InvalidStellarAddress(addr.to_string())
    }

    /// Create a `CommandError` with context.
    pub fn command_error(msg: impl Into<String>) -> Self {
        Self::CommandError(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_stellar_secret_key_error_message() {
        let err = CliError::invalid_secret_key("not-a-key");
        let msg = err.to_string();
        assert!(msg.contains("Invalid Stellar secret key"));
        assert!(msg.contains("S..."));
    }

    #[test]
    fn test_invalid_deadline_error_message() {
        let err = CliError::invalid_deadline("bad-date");
        let msg = err.to_string();
        assert!(msg.contains("Invalid deadline format"));
        assert!(msg.contains("2026-12-31T23:59:59Z"));
    }

    #[test]
    fn test_wasm_file_not_found_error() {
        let err = CliError::wasm_not_found("/nonexistent/file.wasm");
        let msg = err.to_string();
        assert!(msg.contains("WASM file not found"));
        assert!(msg.contains("nonexistent"));
    }

    #[test]
    fn test_file_read_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err = CliError::file_read_error("/sensitive/file", io_err);
        let msg = err.to_string();
        assert!(msg.contains("Failed to read file"));
        assert!(msg.contains("sensitive/file"));
        assert!(msg.contains("access denied"));
    }

    #[test]
    fn test_invalid_contract_id_error() {
        let err = CliError::invalid_contract_id("invalid-id");
        let msg = err.to_string();
        assert!(msg.contains("Invalid Stellar contract ID"));
        assert!(msg.contains("C..."));
    }

    #[test]
    fn test_invalid_input_error() {
        let err = CliError::invalid_input("freelancer address is malformed");
        let msg = err.to_string();
        assert!(msg.contains("Invalid input"));
        assert!(msg.contains("freelancer address is malformed"));
    }

    #[test]
    fn test_rpc_error() {
        let err = CliError::rpc_error("connection timeout after 30s");
        let msg = err.to_string();
        assert!(msg.contains("RPC error"));
        assert!(msg.contains("connection timeout"));
    }

    #[test]
    fn test_config_error() {
        let err = CliError::config_error("duplicate key 'rpc_url' in config file");
        let msg = err.to_string();
        assert!(msg.contains("Configuration error"));
        assert!(msg.contains("duplicate key"));
    }

    #[test]
    fn test_serialization_error() {
        let err = CliError::serialization_error("trailing commas not allowed in JSON");
        let msg = err.to_string();
        assert!(msg.contains("JSON processing error"));
        assert!(msg.contains("trailing commas"));
    }

    #[test]
    fn test_invalid_argument_error() {
        let err = CliError::invalid_argument("--amount must be positive");
        let msg = err.to_string();
        assert!(msg.contains("Missing or invalid argument"));
        assert!(msg.contains("positive"));
    }

    #[test]
    fn test_crypto_error() {
        let err = CliError::crypto_error("signature verification failed");
        let msg = err.to_string();
        assert!(msg.contains("Cryptographic operation failed"));
        assert!(msg.contains("signature verification"));
    }

    #[test]
    fn test_operation_cancelled_error() {
        let err = CliError::operation_cancelled("user declined confirmation prompt");
        let msg = err.to_string();
        assert!(msg.contains("Operation cancelled"));
        assert!(msg.contains("user declined"));
    }

    #[test]
    fn test_contract_execution_error() {
        let err = CliError::contract_execution_error("insufficient balance for payment");
        let msg = err.to_string();
        assert!(msg.contains("Contract execution failed"));
        assert!(msg.contains("insufficient balance"));
    }

    #[test]
    fn test_invalid_stellar_address_error() {
        let err = CliError::invalid_stellar_address("GBADADDRESS");
        let msg = err.to_string();
        assert!(msg.contains("Invalid Stellar address"));
        assert!(msg.contains("G..."));
    }

    #[test]
    fn test_command_error() {
        let err = CliError::command_error("subcommand initialization failed");
        let msg = err.to_string();
        assert!(msg.contains("Command failed"));
        assert!(msg.contains("initialization"));
    }

    #[test]
    fn test_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CliError>();
    }
}
