use keyring::Entry;

const SERVICE_NAME: &str = "pin-client";

pub fn store_credentials(client_id: &str, api_secret: &str) -> Result<(), Box<dyn std::error::Error>> {
    let entry = Entry::new(SERVICE_NAME, client_id)?;
    entry.set_password(api_secret)?;
    log::info!("Credentials stored securely for client: {}", client_id);
    Ok(())
}

pub fn get_credentials(client_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let entry = Entry::new(SERVICE_NAME, client_id)?;
    let password = entry.get_password()?;
    Ok(password)
}

pub fn delete_credentials(client_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let entry = Entry::new(SERVICE_NAME, client_id)?;
    entry.delete_credential()?;
    log::info!("Credentials deleted for client: {}", client_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_credential_roundtrip() {
        let test_id = "test_client_123";
        let test_secret = "test_secret_abc";
        
        store_credentials(test_id, test_secret).unwrap();
        let retrieved = get_credentials(test_id).unwrap();
        assert_eq!(retrieved, test_secret);
        
        delete_credentials(test_id).unwrap();
    }
}
