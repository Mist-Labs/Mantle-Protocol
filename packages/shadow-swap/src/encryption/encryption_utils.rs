use anyhow::{Result, anyhow};
use secp256k1::SecretKey;

pub fn decrypt_with_ecies(encrypted_hex: &str, private_key_hex: &str) -> Result<String> {
    let encrypted_hex = encrypted_hex.strip_prefix("0x").unwrap_or(encrypted_hex);
    let private_key_hex = private_key_hex
        .strip_prefix("0x")
        .unwrap_or(private_key_hex);

    let encrypted =
        hex::decode(encrypted_hex).map_err(|e| anyhow!("Invalid encrypted data hex: {}", e))?;

    let private_key_bytes =
        hex::decode(private_key_hex).map_err(|e| anyhow!("Invalid private key hex: {}", e))?;

    let secret_key = SecretKey::from_slice(&private_key_bytes)
        .map_err(|e| anyhow!("Invalid private key format: {}", e))?;

    let decrypted_bytes = ecies::decrypt(&secret_key.secret_bytes(), &encrypted)
        .map_err(|e| anyhow!("ECIES decryption failed: {}", e))?;

    let hex_string = hex::encode(&decrypted_bytes);
    Ok(format!("0x{}", hex_string))
}

#[cfg(test)]
mod tests {
    use super::*;
    use secp256k1::{PublicKey, Secp256k1};

    #[test]
    fn test_ecies_round_trip() {
        // Generate test keypair
        let private_key_hex = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let private_key_bytes = hex::decode(private_key_hex).unwrap();
        let secret_key = SecretKey::from_slice(&private_key_bytes).unwrap();

        let secp = Secp256k1::new();
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        let public_key_bytes = public_key.serialize();

        // Test data
        let original = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";

        // Encrypt
        let encrypted = ecies::encrypt(&public_key_bytes, original.as_bytes()).unwrap();
        let encrypted_hex = hex::encode(&encrypted);

        // Decrypt
        let decrypted = decrypt_with_ecies(&encrypted_hex, private_key_hex).unwrap();

        assert_eq!(format!("0x{}", original), decrypted);
        println!("✅ Decryption test passed");
    }
}
