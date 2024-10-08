use dotenv::dotenv;
use flowsnet_platform_sdk::logger;
use github_flows::{
    get_octo,
    // event_handler, , listen_to_event,
    octocrab::models::webhook_events::payload::PullRequestWebhookEventPayload,
    // octocrab::models::Author,
    // octocrab::models::webhook_events::payload::{
    //     PullRequestWebhookEventAction, PullRequestWebhookEventPayload,
    // },
    // octocrab::models::webhook_events::{WebhookEvent, WebhookEventPayload},
    // octocrab::models::CommentId,
    GithubLogin,
};
use llmservice_flows::{chat::ChatOptions, LLMServiceFlows};
use reqwest;
use serde_json::{self, Value};
use std::collections::HashMap;
use std::env;
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

    let _ = process_body(&_body).await;
    // match serde_json::to_string_pretty(&load) {
    //     Ok(pretty_json) => log::info!("Received webhook payload: {}", pretty_json),
    //     Err(e) => log::error!("Failed to deserialize webhook payload: {}", e),
    // }
}

pub async fn process_body(_body: &[u8]) {
    let load = match serde_json::from_slice::<PullRequestWebhookEventPayload>(&_body) {
        Ok(obj) => obj,
        Err(_e) => {
            log::error!("failed to parse body: {}", _e);
            panic!("failed to parse body");
        }
    };

    let owner = match load.pull_request.user.as_ref() {
        Some(u) => u.login.to_owned(),
        None => "unknown_user".to_string(),
    };

    let repo = match load.pull_request.repo.as_ref() {
        Some(rep) => rep.name.to_owned(),
        None => "unknown_repo".to_string(),
    };

    let pull_number = load.number;
    let title = load
        .pull_request
        .title
        .unwrap_or("pull request has no title".to_string());

    log::info!("pr owner: {:?}", owner);
    log::info!("repo from pr: {:?}", repo);
    log::info!("pull number: {:?}", pull_number);
    log::info!("pull title: {:?}", title);
    // log::info!("parsed payload: {:?}", load);

    let _ = review_with_llm_octo(&owner, &repo, pull_number, &title).await;
}

pub async fn review_with_llm_octo(owner: &str, repo: &str, pull_number: u64, title: &str) {
    let llm_api_endpoint =
        env::var("llm_api_endpoint").unwrap_or("https://api.openai.com/v1".to_string());
    let llm_model_name = env::var("llm_model_name").unwrap_or("gpt-4o".to_string());
    let llm_ctx_size = env::var("llm_ctx_size")
        .unwrap_or("16384".to_string())
        .parse::<u32>()
        .unwrap_or(0);
    let llm_api_key = env::var("llm_api_key").unwrap_or("LLAMAEDGE".to_string());

    //  The soft character limit of the input context size
    //  This is measured in chars. We set it to be 2x llm_ctx_size, which is measured in tokens.
    let ctx_size_char: usize = (2 * llm_ctx_size).try_into().unwrap_or(0);

    let chat_id = format!("PR#{pull_number}");
    let system = &format!("You are an experienced software developer. You will review a source code file and its patch related to the subject of \"{}\". Please be as concise as possible while being accurate.", title);
    let mut lf = LLMServiceFlows::new(&llm_api_endpoint);
    lf.set_api_key(&llm_api_key);

    let octo = get_octo(&GithubLogin::Default);
    let issues = octo.issues(owner, repo);

    let pulls = octo.pulls(owner, repo);

    let mut resp = String::new();
    resp.push_str("Hello, I am a [code review agent](https://github.com/flows-network/github-pr-review/) on [flows.network](https://flows.network/). Here are my reviews of changed source code files in this PR.\n\n------\n\n");
    match pulls.list_files(pull_number).await {
        Ok(files) => {
            // let client = reqwest::Client::new();
            for f in files.items {
                let filename = &f.filename;
                if filename.ends_with(".md")
                    || filename.ends_with(".js")
                    || filename.ends_with(".css")
                    || filename.ends_with(".html")
                    || filename.ends_with(".htm")
                {
                    continue;
                }

                // The f.raw_url is a redirect. So, we need to construct our own here.
                let contents_url = f.contents_url.as_str();
                if contents_url.len() < 40 {
                    continue;
                }
                let hash = &contents_url[(contents_url.len() - 40)..];
                let raw_url = format!(
                    "https://raw.githubusercontent.com/{owner}/{repo}/{}/{}",
                    hash, filename
                );

                log::debug!("Fetching url: {}", raw_url);
                let res = match reqwest::get(raw_url.as_str()).await {
                    Ok(r) => r,
                    Err(e) => {
                        log::error!("Error fetching file {}: {}", filename, e);
                        continue;
                    }
                };
                // let res = reqwest::get(raw_url.as_str()).await.unwrap();
                // let res = client.get(raw_url.as_str()).send().await.unwrap();
                log::debug!("Fetched file: {}", filename);
                let file_as_text = res.text().await.unwrap();
                let t_file_as_text = truncate(&file_as_text, ctx_size_char);

                resp.push_str("## [");
                resp.push_str(filename);
                resp.push_str("](");
                resp.push_str(f.blob_url.as_str());
                resp.push_str(")\n\n");

                log::debug!("Sending file to LLM: {}", filename);
                let co = ChatOptions {
                    model: Some(&llm_model_name),
                    token_limit: llm_ctx_size,
                    restart: true,
                    system_prompt: Some(system),
                    ..Default::default()
                };
                let question = "Review the following source code and report only major bugs or issues. The most important coding issues should be reported first. You should report NO MORE THAN 3 issues. Be very concise and explain each coding issue in one sentence. The code might be truncated. NEVER comment on the completeness of the source code.\n\n".to_string() + t_file_as_text;
                match lf.chat_completion(&chat_id, &question, &co).await {
                    Ok(r) => {
                        resp.push_str("#### Potential issues");
                        resp.push_str("\n\n");
                        resp.push_str(&r.choice);
                        resp.push_str("\n\n");
                        log::debug!("Received LLM resp for file: {}", filename);
                    }
                    Err(e) => {
                        resp.push_str("#### Potential issues");
                        resp.push_str("\n\n");
                        resp.push_str("N/A");
                        resp.push_str("\n\n");
                        log::error!("LLM returns error for file review for {}: {}", filename, e);
                    }
                }

                log::debug!("Sending patch to LLM: {}", filename);
                let co = ChatOptions {
                    model: Some(&llm_model_name),
                    token_limit: llm_ctx_size,
                    restart: true,
                    system_prompt: Some(system),
                    ..Default::default()
                };
                let patch_as_text = f.patch.unwrap_or("".to_string());
                let t_patch_as_text = truncate(&patch_as_text, ctx_size_char);
                let question = "The following is a change patch for the file. Please summarize key changes in short bullet points. List the most important changes first. You list should contain no more than the top 3 most important changes.  \n\n".to_string() + t_patch_as_text;
                match lf.chat_completion(&chat_id, &question, &co).await {
                    Ok(r) => {
                        resp.push_str("#### Summary of changes");
                        resp.push_str("\n\n");
                        resp.push_str(&r.choice);
                        resp.push_str("\n\n");
                        log::debug!("Received LLM resp for patch: {}", filename);
                    }
                    Err(e) => {
                        resp.push_str("#### Summary of changes");
                        resp.push_str("\n\n");
                        resp.push_str("N/A");
                        resp.push_str("\n\n");
                        log::error!("LLM returns error for patch review for {}: {}", filename, e);
                    }
                }
            }
        }
        Err(_error) => {
            log::error!("Cannot get file list");
        }
    }

    // Send the entire response to GitHub PR
    match issues.create_comment(pull_number, resp).await {
        Err(error) => {
            log::error!("Error posting resp: {}", error);
        }
        _ => {}
    }
}

fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}
