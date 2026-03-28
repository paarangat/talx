use keyring::Entry;

const SERVICE: &str = "com.talx.app";

/// Maps provider identifiers to environment variable names for dev fallback.
fn env_var_name(provider: &str) -> Option<&'static str> {
    match provider {
        "groq" => Some("GROQ_API_KEY"),
        "soniox" => Some("SONIOX_API_KEY"),
        "llm_groq" => Some("GROQ_LLM_API_KEY"),
        "llm_openai" => Some("OPENAI_API_KEY"),
        _ => None,
    }
}

/// Retrieve an API key. Checks environment variables first (dev override),
/// then falls back to the OS keychain. Returns Ok(None) if the key is not
/// found in either location.
pub fn get_key(provider: &str) -> Result<Option<String>, String> {
    // Check env var first (dev mode override)
    if let Some(var_name) = env_var_name(provider) {
        if let Ok(val) = std::env::var(var_name) {
            if !val.is_empty() {
                return Ok(Some(val));
            }
        }
    }

    let entry = Entry::new(SERVICE, provider).map_err(|e| format!("Keychain error: {e}"))?;
    match entry.get_password() {
        Ok(password) => Ok(Some(password)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Failed to read keychain: {e}")),
    }
}

/// Store an API key in the OS keychain. If the key is empty, deletes it instead.
pub fn set_key(provider: &str, key: &str) -> Result<(), String> {
    if key.is_empty() {
        return delete_key(provider);
    }
    let entry = Entry::new(SERVICE, provider).map_err(|e| format!("Keychain error: {e}"))?;
    entry
        .set_password(key)
        .map_err(|e| format!("Failed to write keychain: {e}"))
}

/// Remove an API key from the OS keychain. No-op if the key doesn't exist.
pub fn delete_key(provider: &str) -> Result<(), String> {
    let entry = Entry::new(SERVICE, provider).map_err(|e| format!("Keychain error: {e}"))?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Failed to delete keychain entry: {e}")),
    }
}
