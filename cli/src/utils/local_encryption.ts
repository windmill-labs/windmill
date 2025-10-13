import crypto from "node:crypto";

// Helper function to convert strings to Uint8Array (binary)
function encode(input: string): Uint8Array {
  return new TextEncoder().encode(input);
}

// Helper function to convert Uint8Array (binary) to base64
function toBase64(arr: Uint8Array): string {
  return btoa(String.fromCharCode(...arr));
}

// Helper function to convert base64 to Uint8Array (binary)
function fromBase64(base64: string): Uint8Array {
  return new Uint8Array(
    atob(base64)
      .split("")
      .map((char) => char.charCodeAt(0))
  );
}

// Function to derive a 256-bit key from any input string using SHA-256
async function deriveKey(
  keyString: string
): Promise<crypto.webcrypto.CryptoKey> {
  const keyMaterial = encode(keyString);
  const keyHash = await crypto.subtle.digest("SHA-256", keyMaterial); // Generate SHA-256 hash
  // Import the hash as a CryptoKey for AES-GCM
  return crypto.subtle.importKey("raw", keyHash, { name: "AES-GCM" }, false, [
    "encrypt",
    "decrypt",
  ]);
}

// Encrypt function
export async function encrypt(
  plaintext: string,
  keyString: string
): Promise<string> {
  const key = await deriveKey(keyString); // Derive a 256-bit AES key from any input string
  const iv = crypto.getRandomValues(new Uint8Array(12)); // AES-GCM needs a 12-byte IV
  const encrypted = await crypto.subtle.encrypt(
    {
      name: "AES-GCM",
      iv,
      tagLength: 128,
    },
    key,
    encode(plaintext) // convert plaintext to binary
  );

  // Concatenate IV and encrypted data
  const combined = new Uint8Array(iv.length + encrypted.byteLength);
  combined.set(iv, 0); // first part is the IV
  combined.set(new Uint8Array(encrypted), iv.length); // second part is the ciphertext

  // Convert to base64 for storage/transmission
  return toBase64(combined);
}

// Decrypt function
export async function decrypt(
  combinedCiphertext: string,
  keyString: string
): Promise<string> {
  const key = await deriveKey(keyString); // Derive the same 256-bit AES key from the input string
  const combined = fromBase64(combinedCiphertext); // decode base64 to binary

  // Split the IV and the ciphertext
  const iv = combined.slice(0, 12); // First 12 bytes are the IV
  const ciphertext = combined.slice(12); // The rest is the encrypted data
  console.log();

  // log.info({keyString, key, ciphertext})
  // Perform decryption
  const decrypted = await crypto.subtle.decrypt(
    {
      name: "AES-GCM",
      iv,
      tagLength: 128,
    },
    key,
    ciphertext
  );

  // Convert decrypted data from binary to string
  return new TextDecoder().decode(decrypted);
}

//   // Example usage:
//   const key = "any-length-key-you-want"; // Now can be any length
//   const message = "This is a secret message.";

//   encrypt(message, key).then((combinedCiphertext) => {
//     console.log("Encrypted message:", combinedCiphertext);

//     // Now decrypt it
//     decrypt(combinedCiphertext, key).then((decryptedMessage) => {
//       console.log("Decrypted message:", decryptedMessage);
//     });
//   });
