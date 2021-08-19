use std::path::PathBuf;

use near_crypto::{InMemorySigner, PublicKey};
use near_jsonrpc_client::JsonRpcClient;
use near_jsonrpc_primitives::types::query::{QueryResponseKind, RpcQueryRequest};
use near_primitives::borsh::BorshSerialize;
use near_primitives::hash::CryptoHash;
use near_primitives::transaction::SignedTransaction;
use near_primitives::types::{AccountId, BlockHeight, Finality};
use near_primitives::views::{AccessKeyView, FinalExecutionOutcomeView, QueryRequest};

const SANDBOX_CREDENTIALS_DIR: &str = ".near-credentials/sandbox/";


fn home_dir(port: u16) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("sandbox-{}", port));
    path
}

use std::cell::RefCell;

thread_local! {
    pub static CURRENT_SANDBOX_PORT: RefCell<u16> = RefCell::new(3030);
}

pub(crate) fn sandbox_client() -> JsonRpcClient {
    CURRENT_SANDBOX_PORT.with(|port| {
        near_jsonrpc_client::new_client(&format!("http://localhost:{}", *port.borrow()))
    })
}

pub(crate) fn root_account() -> InMemorySigner {
    let port = CURRENT_SANDBOX_PORT.with(|port| {
        *port.borrow()
    });
    let mut path = home_dir(port);
    path.push("validator_key.json");

    let root_signer = InMemorySigner::from_file(&path);
    root_signer
}

pub(crate) async fn access_key(
    account_id: String,
    pk: PublicKey,
) -> Result<(AccessKeyView, BlockHeight, CryptoHash), String> {
    let query_resp = sandbox_client()
        .query(RpcQueryRequest {
            block_reference: Finality::Final.into(),
            request: QueryRequest::ViewAccessKey {
                account_id,
                public_key: pk,
            },
        })
        .await
        .map_err(|err| format!("Failed to fetch public key info: {:?}", err))?;

    match query_resp.kind {
        QueryResponseKind::AccessKey(access_key) => {
            Ok((access_key, query_resp.block_height, query_resp.block_hash))
        }
        _ => Err("Could not retrieve access key".to_owned()),
    }
}

pub(crate) async fn send_tx(tx: SignedTransaction) -> Result<FinalExecutionOutcomeView, String> {
    let json_rpc_client = sandbox_client();
    let transaction_info_result = loop {
        let transaction_info_result = json_rpc_client
            .broadcast_tx_commit(near_primitives::serialize::to_base64(
                tx.try_to_vec()
                    .expect("Transaction is not expected to fail on serialization"),
            ))
            .await;

        if let Err(ref err) = transaction_info_result {
            if let Some(serde_json::Value::String(data)) = &err.data {
                if data.contains("Timeout") {
                    println!("Error transaction: {:?}", err);
                    continue;
                }
            }
        }

        break transaction_info_result;
    };

    transaction_info_result.map_err(|e| format!("Error transaction: {:?}", e))
}

pub(crate) fn credentials_filepath(account_id: AccountId) -> Result<PathBuf, String> {
    let home_dir = dirs::home_dir().ok_or_else(|| "Could not get HOME_DIR".to_string())?;
    let mut path = PathBuf::from(&home_dir);
    path.push(SANDBOX_CREDENTIALS_DIR);

    // Create this path's directories if they don't exist:
    std::fs::create_dir_all(path.clone())
        .map_err(|e| format!("Could not create near credential directory: {}", e))?;

    path.push(format!("{}.json", account_id));
    Ok(path)
}
