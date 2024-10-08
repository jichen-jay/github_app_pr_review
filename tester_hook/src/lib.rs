use dotenv::dotenv;
use flowsnet_platform_sdk::logger;
// use github_app_pr_review::{fetch_and_review_files};
// use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use webhook_flows::{create_endpoint, request_handler};

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn on_deploy() {
    create_endpoint().await;
}

#[request_handler]
async fn handler(
    _headers: Vec<(String, String)>,
    _subpath: String,
    _qry: HashMap<String, Value>,
    _body: Vec<u8>,
) {
    logger::init();
    dotenv().ok();

    let load = match String::from_utf8(_body) {
        Ok(obj) => obj,
        Err(_e) => {
            log::error!("failed to parse body: {}", _e);
            panic!("failed to parse body");
        }
    };

    // match serde_json::to_string_pretty(&hook_payload_struct) {
    //     Ok(pretty_json) => log::info!("Received webhook payload: {}", pretty_json),
    //     Err(e) => log::error!("Failed to deserialize webhook payload: {}", e),
    // }

    log::info!("payload dump: {:?}", load);


}
