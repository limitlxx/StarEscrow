/// Mock Soroban RPC server tests using wiremock.
///
/// These tests spin up a local HTTP server and verify that `RpcClient`
/// correctly serialises requests and deserialises responses without
/// touching the real network.
#[cfg(test)]
mod tests {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::rpc::RpcClient;

    fn simulate_ok_body() -> serde_json::Value {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "minResourceFee": "12345",
                "results": []
            }
        })
    }

    fn send_ok_body() -> serde_json::Value {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "hash": "abcdef1234567890",
                "status": "PENDING"
            }
        })
    }

    fn get_tx_success_body() -> serde_json::Value {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "status": "SUCCESS",
                "resultXdr": "AAAA",
                "resultMetaXdr": "BBBB"
            }
        })
    }

    // ── simulateTransaction ──────────────────────────────────────────────────

    #[tokio::test]
    async fn simulate_transaction_mocked() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(simulate_ok_body()))
            .mount(&server)
            .await;

        let client = RpcClient::new(&server.uri());
        let result = client.simulate_transaction("AAAA").unwrap();

        assert_eq!(result.min_resource_fee.as_deref(), Some("12345"));
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn simulate_transaction_error_field() {
        let server = MockServer::start().await;

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "error": "contract trap"
            }
        });

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let client = RpcClient::new(&server.uri());
        let result = client.simulate_transaction("AAAA").unwrap();

        assert_eq!(result.error.as_deref(), Some("contract trap"));
    }

    // ── sendTransaction ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn send_transaction_mocked() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(send_ok_body()))
            .mount(&server)
            .await;

        let client = RpcClient::new(&server.uri());
        let result = client.send_transaction("AAAA").unwrap();

        assert_eq!(result.hash, "abcdef1234567890");
        assert_eq!(result.status, "PENDING");
        assert!(result.error_result_xdr.is_none());
    }

    #[tokio::test]
    async fn send_transaction_error_status() {
        let server = MockServer::start().await;

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "hash": "deadbeef",
                "status": "ERROR",
                "errorResultXdr": "ZZZZ"
            }
        });

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let client = RpcClient::new(&server.uri());
        let result = client.send_transaction("AAAA").unwrap();

        assert_eq!(result.status, "ERROR");
        assert_eq!(result.error_result_xdr.as_deref(), Some("ZZZZ"));
    }

    // ── getTransaction / poll ────────────────────────────────────────────────

    #[tokio::test]
    async fn get_transaction_success_mocked() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(get_tx_success_body()))
            .mount(&server)
            .await;

        let client = RpcClient::new(&server.uri());
        let result = client.get_transaction("abcdef1234567890").unwrap();

        assert_eq!(result.status, "SUCCESS");
        assert_eq!(result.result_xdr.as_deref(), Some("AAAA"));
    }

    #[tokio::test]
    async fn poll_transaction_resolves_after_not_found() {
        let server = MockServer::start().await;

        // First call returns NOT_FOUND, second returns SUCCESS.
        let not_found = serde_json::json!({
            "jsonrpc": "2.0", "id": 1,
            "result": { "status": "NOT_FOUND" }
        });

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(not_found))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(get_tx_success_body()))
            .mount(&server)
            .await;

        let client = RpcClient::new(&server.uri());
        let result = client.poll_transaction("abcdef1234567890", 3, 0).unwrap();

        assert_eq!(result.status, "SUCCESS");
    }

    #[tokio::test]
    async fn poll_transaction_exhausts_attempts() {
        let server = MockServer::start().await;

        let not_found = serde_json::json!({
            "jsonrpc": "2.0", "id": 1,
            "result": { "status": "NOT_FOUND" }
        });

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(not_found))
            .mount(&server)
            .await;

        let client = RpcClient::new(&server.uri());
        let result = client.poll_transaction("deadbeef", 2, 0);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found after"));
    }
}
