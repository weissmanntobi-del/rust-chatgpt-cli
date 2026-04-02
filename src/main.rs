use anyhow::{anyhow, Context, Result};
use clap::Parser;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Parser, Debug)]
#[command(name = "chatgpt-cli")]
#[command(about = "A simple Rust CLI for OpenAI Responses API")]
struct Args {
    /// Prompt to send to the model
    #[arg(short, long)]
    prompt: String,

    /// Model name
    #[arg(short, long, default_value = "gpt-4o-mini")]
    model: String,

    /// Optional system-style instructions
    #[arg(long)]
    instructions: Option<String>,
}

#[derive(Serialize)]
struct ResponseRequest<'a> {
    model: &'a str,
    input: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    instructions: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    output: Option<Vec<OutputItem>>,
}

#[derive(Debug, Deserialize)]
struct OutputItem {
    #[serde(rename = "type")]
    item_type: String,
    content: Option<Vec<ContentItem>>,
}

#[derive(Debug, Deserialize)]
struct ContentItem {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

fn extract_text(resp: &ApiResponse) -> String {
    let mut out = String::new();

    if let Some(items) = &resp.output {
        for item in items {
            if item.item_type == "message" {
                if let Some(content) = &item.content {
                    for c in content {
                        if c.content_type == "output_text" {
                            if let Some(text) = &c.text {
                                out.push_str(text);
                            }
                        }
                    }
                }
            }
        }
    }

    out
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let api_key = env::var("OPENAI_API_KEY")
        .context("OPENAI_API_KEY is not set")?;

    let body = ResponseRequest {
        model: &args.model,
        input: &args.prompt,
        instructions: args.instructions.as_deref(),
    };

    let client = reqwest::Client::new();

    let res = client
        .post("https://api.openai.com/v1/responses")
        .header(CONTENT_TYPE, "application/json")
        .header(AUTHORIZATION, format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await
        .context("failed to call OpenAI API")?;

    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        return Err(anyhow!("API error {}: {}", status, text));
    }

    let parsed: ApiResponse = res.json().await.context("invalid JSON from API")?;
    let text = extract_text(&parsed);

    if text.is_empty() {
        return Err(anyhow!("No text output found in response"));
    }

    println!("{}", text);
    Ok(())
}
