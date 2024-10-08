use dotenv::dotenv;
use flowsnet_platform_sdk::logger;
// use github_app_pr_review::{fetch_and_review_files};
// use serde::{Deserialize, Serialize};
use github_flows::{
    // event_handler, get_octo, listen_to_event,
    octocrab::models::webhook_events::payload::PullRequestWebhookEventPayload,
    // octocrab::models::Author,
    // octocrab::models::webhook_events::payload::{
    //     PullRequestWebhookEventAction, PullRequestWebhookEventPayload,
    // },
    // octocrab::models::webhook_events::{WebhookEvent, WebhookEventPayload},
    // octocrab::models::CommentId,
    // GithubLogin,
};
use serde_json::{self, Value};
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

    // let load = match String::from_utf8(_body) {
    //     Ok(obj) => obj,
    //     Err(_e) => {
    //         log::error!("failed to parse body: {}", _e);
    //         panic!("failed to parse body");
    //     }
    // };

    // log::info!("payload dump: {:?}", load);

    let _ = log_them(&_body).await;
    // match serde_json::to_string_pretty(&load) {
    //     Ok(pretty_json) => log::info!("Received webhook payload: {}", pretty_json),
    //     Err(e) => log::error!("Failed to deserialize webhook payload: {}", e),
    // }
}

// pub async fn parse_incoming_payload() {

//     let octo = get_octo(&GithubLogin::Default);
//     let issues = octo.issues(owner.clone(), repo.clone());

//     let pulls = octo.pulls(owner.clone(), repo.clone());

// }

pub async fn log_them(_body: &[u8]) {
    let load = match serde_json::from_slice::<PullRequestWebhookEventPayload>(&_body) {
        Ok(obj) => obj,
        Err(_e) => {
            log::error!("failed to parse body: {}", _e);
            panic!("failed to parse body");
        }
    };

    let owner = load.pull_request.user.as_ref()
        .and_then(|user| Some(user.login.as_ref()))
        .unwrap_or("unknown_user"); // Provide a default if none

    let repo = load.pull_request.repo.as_ref()
        .and_then(|repository| Some(repository.name.as_ref()))
        .unwrap_or("unknown_repo"); // Provide a default if none

    // let owner_what = load.pull_request.repo.as_ref()
    //     .and_then(|repository| Some(repository.owner.as_ref()))
    //     .unwrap_or("failed to get author");

    // let repo_what = load.pull_request.user.as_ref()
    // .and_then(|user| Some(user.repos_url.as_ref()))
    //     .unwrap_or("unknown_repos_url"); // Provide a default if none

    let pull_number = load.number;
    


    log::info!("pr owner: {:?}", owner);
    log::info!("repo from pr: {:?}", repo);
    // log::info!("owner what: {:?}", owner_what);
    // log::info!("repo what: {:?}", repo_what);
    log::info!("pull number: {:?}", pull_number);
    log::info!("parsed payload: {:?}", load);
}
