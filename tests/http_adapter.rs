use dreaming::{
    provider::{GenerationRequest, ModelProvider, OpenAiCompatible, ProviderCapabilities},
    types::Config,
};
use std::io::{Read, Write};
fn server(status: &str, body: String) -> (String, std::thread::JoinHandle<String>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let status = status.to_owned();
    let thread = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut bytes = Vec::new();
        let mut buf = [0; 4096];
        loop {
            let n = stream.read(&mut buf).unwrap();
            if n == 0 {
                break;
            }
            bytes.extend_from_slice(&buf[..n]);
            if let Some(end) = bytes.windows(4).position(|b| b == b"\r\n\r\n") {
                let header = String::from_utf8_lossy(&bytes[..end]);
                let len = header
                    .lines()
                    .find_map(|l| {
                        l.to_lowercase()
                            .strip_prefix("content-length: ")
                            .and_then(|s| s.parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                if bytes.len() >= end + 4 + len {
                    break;
                }
            }
        }
        write!(stream,"HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
        String::from_utf8(bytes).unwrap()
    });
    (format!("http://{address}/v1/chat/completions"), thread)
}
fn request() -> GenerationRequest {
    GenerationRequest {
        system: "fixed policy".into(),
        prompt: "task data".into(),
        config: Config {
            seed: Some(42),
            ..Config::default()
        },
    }
}
#[tokio::test]
async fn adapter_sends_roles_and_records_usage_without_seed_assumptions() {
    let (url,thread)=server("200 OK",r#"{"choices":[{"message":{"content":"6"},"finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":1}}"#.into());
    let provider = OpenAiCompatible::new(
        &url,
        Some("test-placeholder".into()),
        ProviderCapabilities::default(),
    )
    .unwrap();
    let response = provider.generate(&request()).await.unwrap();
    assert_eq!(response.content, "6");
    assert_eq!(response.prompt_tokens, Some(10));
    let received = thread.join().unwrap();
    let body: serde_json::Value =
        serde_json::from_str(received.split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(body["messages"][0]["role"], "system");
    assert_eq!(body["messages"][1]["role"], "user");
    assert!(body.get("seed").is_none());
    assert!(!body.to_string().contains("test-placeholder"));
}
#[tokio::test]
async fn adapter_rejects_large_responses_and_does_not_echo_error_bodies() {
    let (url, thread) = server("200 OK", "x".repeat(1000));
    let provider = OpenAiCompatible::new(&url, None, ProviderCapabilities::default()).unwrap();
    let mut req = request();
    req.config.max_response_bytes = 64;
    assert!(provider
        .generate(&req)
        .await
        .unwrap_err()
        .contains("byte limit"));
    thread.join().unwrap();
    let (url, thread) = server("401 Unauthorized", "sensitive-server-details".into());
    let provider = OpenAiCompatible::new(&url, None, ProviderCapabilities::default()).unwrap();
    let error = provider.generate(&request()).await.unwrap_err();
    assert!(error.contains("401"));
    assert!(!error.contains("sensitive"));
    thread.join().unwrap();
}
