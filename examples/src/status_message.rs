use serde_json::json;
use workspaces::prelude::*;

const STATUS_MSG_WASM_FILEPATH: &str = "./examples/res/status_message.wasm";

#[tokio::main]
async fn main() {
    let worker = workspaces::sandbox();
    let contract = worker.dev_deploy(STATUS_MSG_WASM_FILEPATH).await.unwrap();

    let outcome = worker
        .call(
            &contract,
            "set_status".into(),
            json!({
                "message": "hello_world",
            })
            .to_string()
            .into_bytes(),
            None,
        )
        .await
        .unwrap();
    println!("set_status: {:?}", outcome);

    let result = worker
        .view(
            contract.id(),
            "get_status".into(),
            json!({
                "account_id": contract.id(),
            })
            .to_string()
            .into_bytes()
            .into(),
        )
        .await
        .unwrap();

    println!(
        "status: {:?}",
        serde_json::to_string_pretty(&result).unwrap()
    );
}
