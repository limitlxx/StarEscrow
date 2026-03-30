use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

/// Blocking Soroban RPC client used by the CLI.
pub struct RpcClient {
    client: Client,
    pub(crate) url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokeResponse {
    pub status: String,
    pub transaction_hash: Option<String>,
    pub result: Value,
}

impl RpcClient {
    pub fn new(url: &str) -> Self {
        Self {
            client: Client::new(),
            url: url.to_owned(),
        }
    }

    /// Invokes a contract function directly via Soroban JSON-RPC.
    pub fn invoke_contract(
        &self,
        contract_id: &str,
        source_secret: &str,
        function: &str,
        args: &Map<String, Value>,
        network_passphrase: &str,
        sim_only: bool,
    ) -> Result<InvokeResponse> {
        let payload = build_invoke_payload(
            contract_id,
            source_secret,
            function,
            args,
            network_passphrase,
            sim_only,
        );
        let response = self.call(payload).context("contract invocation failed")?;
        let result = response.get("result").cloned().unwrap_or_else(|| json!({}));
        let status = result
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or(if sim_only { "SIMULATED" } else { "SUBMITTED" })
            .to_owned();
        let transaction_hash = result
            .get("hash")
            .and_then(Value::as_str)
            .map(str::to_owned);

        Ok(InvokeResponse {
            status,
            transaction_hash,
            result,
        })
    }

    pub fn query_contract(
        &self,
        contract_id: &str,
        function: &str,
        args: &Map<String, Value>,
        network_passphrase: &str,
    ) -> Result<Value> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "invokeContract",
            "params": {
                "contractId": contract_id,
                "function": function,
                "args": args,
                "networkPassphrase": network_passphrase,
                "readOnly": true,
            }
        });
        let response = self.call(payload).context("contract query failed")?;
        Ok(response.get("result").cloned().unwrap_or(Value::Null))
    }

    pub fn get_events(&self, contract_id: &str) -> Result<Vec<Value>> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getEvents",
            "params": {
                "filters": [{"contractIds": [contract_id]}],
                "limit": 200
            }
        });
        let response = self
            .call(payload)
            .context("fetching contract events failed")?;
        let events = response["result"]["events"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        Ok(events)
    }

    pub fn get_contract_wasm_hash(&self, contract_id: &str) -> Result<String> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getContractCode",
            "params": {"contractId": contract_id}
        });
        let response = self
            .call(payload)
            .context("fetching on-chain contract code failed")?;
        let hash = response["result"]["wasmHash"]
            .as_str()
            .context("missing wasmHash in getContractCode response")?;
        Ok(hash.to_owned())
    }

    fn call(&self, body: Value) -> Result<Value> {
        let resp = self
            .client
            .post(&self.url)
            .json(&body)
            .send()
            .context("RPC request failed")?;

        if !resp.status().is_success() {
            bail!(
                "RPC HTTP error {} while calling {}",
                resp.status(),
                body["method"]
            );
        }

        let json: Value = resp.json().context("failed to parse RPC JSON response")?;
        if let Some(err) = json.get("error") {
            bail!("RPC error from {}: {}", body["method"], err);
        }

        Ok(json)
    }
}

pub fn build_invoke_payload(
    contract_id: &str,
    source_secret: &str,
    function: &str,
    args: &Map<String, Value>,
    network_passphrase: &str,
    sim_only: bool,
) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "invokeContract",
        "params": {
            "contractId": contract_id,
            "source": source_secret,
            "function": function,
            "args": args,
            "networkPassphrase": network_passphrase,
            "simulate": sim_only,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_invoke_payload_contains_expected_fields() {
        let mut args = Map::new();
        args.insert("amount".into(), json!(100));
        let payload = build_invoke_payload(
            "C123",
            "S123",
            "create",
            &args,
            "Test SDF Network ; September 2015",
            true,
        );

        assert_eq!(payload["method"], "invokeContract");
        assert_eq!(payload["params"]["contractId"], "C123");
        assert_eq!(payload["params"]["source"], "S123");
        assert_eq!(payload["params"]["function"], "create");
        assert_eq!(payload["params"]["args"]["amount"], 100);
        assert_eq!(payload["params"]["simulate"], true);
    }

    #[test]
    fn test_new_client_keeps_url() {
        let client = RpcClient::new("https://soroban-testnet.stellar.org");
        assert_eq!(client.url, "https://soroban-testnet.stellar.org");
    }

    #[test]
    fn test_rpc_network_error_is_descriptive() {
        let client = RpcClient::new("http://127.0.0.1:1");
        let result = client.get_events("CABC");
        let err = result.expect_err("should fail without RPC server");
        assert!(
            err.to_string().contains("fetching contract events failed"),
            "unexpected error: {err}"
        );
    }
}
