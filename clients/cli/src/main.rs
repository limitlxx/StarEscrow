use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::{json, Map, Value};
use tabled::{Table, Tabled};

mod conf;
mod deadline;
mod error;
mod keypair;
mod keystore;
mod native_xdr;
mod progress;
mod prompt;
mod rpc;
mod wasm_hash;
mod xdr;

pub use error::CliError;

use rpc::RpcClient;

#[derive(Parser)]
#[command(name = "star-escrow", version, about)]
struct Cli {
    #[arg(long, value_name = "FILE")]
    config: Option<std::path::PathBuf>,

    #[arg(long, value_enum, conflicts_with_all = ["rpc_url", "network_passphrase"])]
    network: Option<Network>,

    #[arg(long)]
    rpc_url: Option<String>,

    #[arg(long)]
    network_passphrase: Option<String>,

    #[arg(long, global = true)]
    json: bool,

    #[arg(long, global = true)]
    dry_run: bool,

    #[arg(long, global = true)]
    no_confirm: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::ValueEnum, Clone)]
enum Network {
    Testnet,
    Mainnet,
    Futurenet,
}

impl Network {
    fn rpc_url(&self) -> &'static str {
        match self {
            Network::Testnet => "https://soroban-testnet.stellar.org",
            Network::Mainnet => "https://soroban-mainnet.stellar.org",
            Network::Futurenet => "https://rpc-futurenet.stellar.org",
        }
    }

    fn passphrase(&self) -> &'static str {
        match self {
            Network::Testnet => "Test SDF Network ; September 2015",
            Network::Mainnet => "Public Global Stellar Network ; September 2015",
            Network::Futurenet => "Test SDF Future Network ; October 2022",
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    Init {
        #[arg(long, env = "ESCROW_CONTRACT_ID")]
        contract_id: String,
        #[arg(long, env = "ADMIN_SECRET")]
        admin_secret: String,
        #[arg(long, default_value = "0")]
        fee_bps: u32,
        #[arg(long)]
        fee_collector: String,
    },
    Pause {
        #[arg(long, env = "ESCROW_CONTRACT_ID")]
        contract_id: String,
        #[arg(long, env = "ADMIN_SECRET")]
        admin_secret: String,
    },
    Unpause {
        #[arg(long, env = "ESCROW_CONTRACT_ID")]
        contract_id: String,
        #[arg(long, env = "ADMIN_SECRET")]
        admin_secret: String,
    },
    Create {
        #[arg(long, env = "ESCROW_CONTRACT_ID")]
        contract_id: String,
        #[arg(long, env = "PAYER_SECRET")]
        payer_secret: String,
        #[arg(long)]
        freelancer: String,
        #[arg(long)]
        token: String,
        #[arg(long)]
        amount: i128,
        #[arg(long)]
        milestone: String,
        #[arg(long)]
        deadline: Option<String>,
    },
    SubmitWork {
        #[arg(long, env = "ESCROW_CONTRACT_ID")]
        contract_id: String,
        #[arg(long, env = "FREELANCER_SECRET")]
        freelancer_secret: String,
    },
    TransferFreelancer {
        #[arg(long, env = "ESCROW_CONTRACT_ID")]
        contract_id: String,
        #[arg(long, env = "FREELANCER_SECRET")]
        freelancer_secret: String,
        #[arg(long)]
        new_freelancer: String,
    },
    Approve {
        #[arg(long, env = "ESCROW_CONTRACT_ID")]
        contract_id: String,
        #[arg(long, env = "PAYER_SECRET")]
        payer_secret: String,
    },
    Cancel {
        #[arg(long, env = "ESCROW_CONTRACT_ID")]
        contract_id: String,
        #[arg(long, env = "PAYER_SECRET")]
        payer_secret: String,
    },
    Expire {
        #[arg(long, env = "ESCROW_CONTRACT_ID")]
        contract_id: String,
        #[arg(long, env = "PAYER_SECRET")]
        payer_secret: String,
    },
    Status {
        #[arg(long, env = "ESCROW_CONTRACT_ID")]
        contract_id: String,
    },
    /// Watch a contract's status, polling periodically
    Watch {
        #[arg(long, env = "ESCROW_CONTRACT_ID")]
        contract_id: String,
        #[arg(long, default_value_t = 5)]
        interval: u64,
    },
    List {
        #[arg(long, env = "ESCROW_CONTRACT_ID")]
        contract_id: String,
        #[arg(long)]
        payer: String,
    },
    Verify {
        #[arg(long, env = "ESCROW_CONTRACT_ID")]
        contract_id: String,
        #[arg(long)]
        wasm: std::path::PathBuf,
        #[arg(long)]
        local_only: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = conf::AppConfig::load(cli.config.as_deref())?;
    let (rpc_url, network_passphrase) = resolve_network(&cli, &cfg);
    let client = RpcClient::new(&rpc_url);

    match cli.command {
        Commands::Init {
            contract_id,
            admin_secret,
            fee_bps,
            fee_collector,
        } => {
            let admin_addr = keypair::public_address_from_secret(&admin_secret)
                .map_err(|e| CliError::invalid_secret_key(&format!("ADMIN_SECRET: {}", e)))?;
            invoke_write(
                &client,
                &rpc_url,
                &network_passphrase,
                &contract_id,
                &admin_secret,
                "init",
                args(&[
                    ("admin", json!(admin_addr)),
                    ("fee_bps", json!(fee_bps)),
                    ("fee_collector", json!(fee_collector)),
                ]),
                cli.dry_run,
                cli.json,
            )
            .await?;
        },
        Commands::Pause {
            contract_id,
            admin_secret,
        } => invoke_write(
            &client,
            &rpc_url,
            &network_passphrase,
            &contract_id,
            &admin_secret,
            "pause",
            Map::new(),
            cli.dry_run,
            cli.json,
        )
        .await?,
        Commands::Unpause {
            contract_id,
            admin_secret,
        } => invoke_write(
            &client,
            &rpc_url,
            &network_passphrase,
            &contract_id,
            &admin_secret,
            "unpause",
            Map::new(),
            cli.dry_run,
            cli.json,
        )
        .await?,
        Commands::Create {
            contract_id,
            payer_secret,
            freelancer,
            token,
            amount,
            milestone,
            deadline,
        } => {
            let payer_addr = keypair::public_address_from_secret(&payer_secret)
                .map_err(|e| CliError::invalid_secret_key(&format!("PAYER_SECRET: {}", e)))?;
            let deadline_ts = parse_deadline(deadline)?;
            invoke_write(
                &client,
                &rpc_url,
                &network_passphrase,
                &contract_id,
                &payer_secret,
                "create",
                args(&[
                    ("payer", json!(payer_addr)),
                    ("freelancer", json!(freelancer)),
                    ("token", json!(token)),
                    ("amount", json!(amount)),
                    ("milestone", json!(milestone)),
                    ("deadline", json!(deadline_ts)),
                ]),
                cli.dry_run,
                cli.json,
            )
            .await?;
        },
        Commands::SubmitWork {
            contract_id,
            freelancer_secret,
        } => invoke_write(
            &client,
            &rpc_url,
            &network_passphrase,
            &contract_id,
            &freelancer_secret,
            "submit_work",
            Map::new(),
            cli.dry_run,
            cli.json,
        )
        .await?,
        Commands::TransferFreelancer {
            contract_id,
            freelancer_secret,
            new_freelancer,
        } => invoke_write(
            &client,
            &rpc_url,
            &network_passphrase,
            &contract_id,
            &freelancer_secret,
            "transfer_freelancer",
            args(&[("new_freelancer", json!(new_freelancer))]),
            cli.dry_run,
            cli.json,
        )
        .await?,
        Commands::Approve {
            contract_id,
            payer_secret,
        } => {
            let confirmed: bool = tokio::task::spawn_blocking(move || {
                prompt::confirm(
                    "Approve milestone and release payment to freelancer?",
                    cli.no_confirm,
                )
            })
            .await
            .expect("spawn_blocking failed")?;
            if !confirmed {
                println!("Aborted.");
                return Ok(());
            }
            invoke_write(
                &client,
                &rpc_url,
                &network_passphrase,
                &contract_id,
                &payer_secret,
                "approve",
                Map::new(),
                cli.dry_run,
                cli.json,
            )
            .await?;
        },
        Commands::Cancel {
            contract_id,
            payer_secret,
        } => {
            let confirmed: bool = tokio::task::spawn_blocking(move || {
                prompt::confirm("Cancel escrow and refund payer?", cli.no_confirm)
            })
            .await
            .expect("spawn_blocking failed")?;
            if !confirmed {
                println!("Aborted.");
                return Ok(());
            }
            invoke_write(
                &client,
                &rpc_url,
                &network_passphrase,
                &contract_id,
                &payer_secret,
                "cancel",
                Map::new(),
                cli.dry_run,
                cli.json,
            )
            .await?;
        },
        Commands::Expire {
            contract_id,
            payer_secret,
        } => invoke_write(
            &client,
            &rpc_url,
            &network_passphrase,
            &contract_id,
            &payer_secret,
            "expire",
            Map::new(),
            cli.dry_run,
            cli.json,
        )
        .await?,
        Commands::Status { contract_id } => {
            let value = client
                .query_contract(&contract_id, "get_escrow", &Map::new(), &network_passphrase)
                .await?;
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"status": "ok", "escrow": value}))?
                );
            } else {
                print_status_table(&value);
            }
        },
        Commands::Watch { contract_id, interval } => {
            let mut interval = interval;
            loop {
                let value = client
                    .query_contract(&contract_id, "get_escrow", &Map::new(), &network_passphrase)
                    .await?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&json!({"status": "ok", "escrow": value}))?);
                } else {
                    print_status_table(&value);
                }
                tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
            }
        },
        Commands::List { contract_id, payer } => {
            let events = client.get_events(&contract_id).await?;
            let filtered: Vec<Value> = events
                .into_iter()
                .filter(|e| e.to_string().contains(&payer))
                .collect();
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"escrows": filtered}))?
                );
            } else {
                println!(
                    "Escrows for payer {payer}:\n{}",
                    Table::new(
                        filtered
                            .iter()
                            .enumerate()
                            .map(|(i, e)| EscrowRow {
                                index: i + 1,
                                event: e.to_string()
                            })
                            .collect::<Vec<_>>()
                    )
                );
            }
        },
        Commands::Verify {
            contract_id,
            wasm,
            local_only,
        } => {
            let local_hash = wasm_hash::hash_wasm_file(&wasm)?;
            if local_only {
                output(
                    cli.json,
                    json!({"local_hash": local_hash}),
                    &format!("Local hash: {local_hash}"),
                );
            } else {
                let remote_hash = client.get_contract_wasm_hash(&contract_id).await?;
                output(
                    cli.json,
                    json!({"local_hash": local_hash, "remote_hash": remote_hash, "match": local_hash.eq_ignore_ascii_case(&remote_hash)}),
                    &format!("Local: {local_hash}\nRemote: {remote_hash}"),
                );
            }
        },
    }

    Ok(())
}

fn resolve_network(cli: &Cli, cfg: &conf::AppConfig) -> (String, String) {
    match &cli.network {
        Some(net) => (net.rpc_url().to_string(), net.passphrase().to_string()),
        None => (
            cli.rpc_url
                .clone()
                .or(cfg.rpc_url.clone())
                .unwrap_or_else(|| "https://soroban-testnet.stellar.org".to_string()),
            cli.network_passphrase
                .clone()
                .or(cfg.network_passphrase.clone())
                .unwrap_or_else(|| "Test SDF Network ; September 2015".to_string()),
        ),
    }
}

fn args(items: &[(&str, Value)]) -> Map<String, Value> {
    items
        .iter()
        .map(|(k, v)| ((*k).to_string(), v.clone()))
        .collect()
}

fn parse_deadline(deadline: Option<String>) -> Result<Option<u64>> {
    match deadline {
        None => Ok(None),
        Some(raw) => raw
            .parse::<u64>()
            .ok()
            .map(Some)
            .map(Ok)
            .unwrap_or_else(|| {
                deadline::parse_iso8601_to_timestamp(&raw)
                    .map(Some)
                    .map_err(|_| CliError::invalid_deadline(&raw).into())
            }),
    }
}

async fn invoke_write(
    client: &RpcClient,
    _rpc_url: &str,
    network_passphrase: &str,
    contract_id: &str,
    source_secret: &str,
    function: &str,
    args: Map<String, Value>,
    dry_run: bool,
    as_json: bool,
) -> Result<()> {
    let response = client
        .invoke_contract(
            contract_id,
            source_secret,
            function,
            &args,
            network_passphrase,
            dry_run,
        )
        .await
        .with_context(|| format!("operation '{function}' failed for contract {contract_id}"))?;

    let human = if dry_run {
        format!("Simulated {function}. Status: {}", response.status)
    } else {
        format!("Submitted {function}. Status: {}", response.status)
    };

    output(
        as_json,
        json!({"status": response.status, "tx_hash": response.transaction_hash, "result": response.result}),
        &human,
    );
    Ok(())
}

fn output(as_json: bool, data: Value, human: &str) {
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&data).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!("{human}");
    }
}

#[derive(Tabled)]
struct StatusRow {
    field: String,
    value: String,
}

fn print_status_table(value: &Value) {
    if let Some(obj) = value.as_object() {
        let rows: Vec<StatusRow> = obj
            .iter()
            .map(|(k, v)| StatusRow {
                field: k.clone(),
                value: v.to_string(),
            })
            .collect();
        println!("{}", Table::new(rows));
    } else {
        println!("{value}");
    }
}

#[derive(Tabled)]
struct EscrowRow {
    #[tabled(rename = "#")]
    index: usize,
    event: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deadline_parsing_timestamp() {
        assert_eq!(
            parse_deadline(Some("1735689600".to_string())).unwrap(),
            Some(1735689600)
        );
    }

    #[test]
    fn test_deadline_parsing_iso8601() {
        assert_eq!(
            parse_deadline(Some("2025-01-01T00:00:00Z".to_string())).unwrap(),
            Some(1735689600)
        );
    }

    #[test]
    fn test_deadline_parsing_invalid_is_descriptive() {
        let err = parse_deadline(Some("bad-date".to_string())).expect_err("must fail");
        let msg = err.to_string();
        assert!(msg.contains("Invalid deadline format"), "Error message should mention 'Invalid deadline format': {}", msg);
        assert!(msg.contains("bad-date"), "Error message should include the invalid input: {}", msg);
    }

    #[test]
    fn test_invalid_deadline_type_shows_cli_error() {
        let err = parse_deadline(Some("!@#$%".to_string())).expect_err("must fail");
        assert!(err.to_string().contains("Invalid deadline"));
    }
}
