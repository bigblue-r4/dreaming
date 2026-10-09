//! Conservative screening for obvious credential material, not a complete DLP system.
use crate::types::Result;
pub fn screen(text: &str) -> Result<()> {
    let lower = text.to_ascii_lowercase();
    if [
        "-----begin private key",
        "-----begin rsa private key",
        "authorization: bearer ",
        "api_key=",
        "apikey=",
        "password=",
        "access_token=",
        "github_pat_",
        "ghp_",
        "xoxb-",
        "sk-ant-",
    ]
    .iter()
    .any(|p| lower.contains(p))
    {
        return Err("credential-like material refused; sanitize experiment data".into());
    }
    Ok(())
}
