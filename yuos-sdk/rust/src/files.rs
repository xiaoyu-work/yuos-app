//! Root-brokered access to one explicitly authorized existing App MCP file.

use base64::Engine as _;

use crate::BridgeError;

/// Maximum file size (32 MiB) supported by brokered file snapshots and replacements.
pub const MAX_FILE_BYTES: usize = 32 * 1024 * 1024;
const CHUNK_BYTES: usize = 512 * 1024;

/// A complete binary snapshot and an opaque revision for this invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSnapshot {
    pub content: Vec<u8>,
    pub revision: String,
}

fn decode_err(verb: &str, message: impl Into<String>) -> BridgeError {
    BridgeError::Decode {
        app: "__app_file".into(),
        verb: verb.into(),
        message: message.into(),
    }
}

fn validate_path(path: &str) -> Result<(), BridgeError> {
    if !path.starts_with('/')
        || path.len() > 4096
        || path[1..]
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || path
            .chars()
            .any(|c| c.is_control() || "*?[]{}<>|;&$`\\".contains(c))
    {
        return Err(decode_err(
            "transfer",
            "brokered file requires an absolute literal file path",
        ));
    }
    Ok(())
}

fn is_valid_token(token: &str) -> bool {
    token.len() == 32 && token.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn is_valid_sha256(digest: &str) -> bool {
    digest.len() == 71
        && digest.starts_with("sha256:")
        && digest[7..]
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (data.len() as u64) * 8;
    let mut padded = data.to_vec();
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in padded.as_chunks::<64>().0 {
        let mut w = [0u32; 64];
        for (i, word) in chunk.as_chunks::<4>().0.iter().enumerate() {
            w[i] = u32::from_be_bytes(*word);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut s = String::with_capacity(71);
    s.push_str("sha256:");
    for word in h {
        use std::fmt::Write as _;
        let _ = write!(&mut s, "{word:08x}");
    }
    s
}

fn call_file_broker(
    commit: bool,
    body: &serde_json::Value,
) -> Result<serde_json::Value, BridgeError> {
    let sub = if commit { "commit" } else { "transfer" };
    let payload = serde_json::to_vec(body).map_err(|e| decode_err(sub, e.to_string()))?;
    crate::yu_call_json_stdin("__app_file", sub, ["__app_file", sub], &payload, false)
}

fn extract_token(res: &serde_json::Value, key: &str) -> Result<String, BridgeError> {
    let token = res
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    if !is_valid_token(token) {
        return Err(decode_err(
            "transfer",
            "broker returned an invalid file transfer token",
        ));
    }
    Ok(token.to_owned())
}

fn abort_transfer(path: &str, token: &str) {
    let _ = call_file_broker(
        false,
        &serde_json::json!({
            "path": path,
            "action": "abort",
            "id": token,
        }),
    );
}

/// Read an existing file in a `brokered-file` MCP invocation.
pub fn read_file(path: &str) -> Result<FileSnapshot, BridgeError> {
    validate_path(path)?;
    let metadata = call_file_broker(
        false,
        &serde_json::json!({
            "path": path,
            "action": "read-start",
        }),
    )?;
    let revision = extract_token(&metadata, "revision")?;
    let result = (|| {
        let size = metadata
            .get("size")
            .and_then(serde_json::Value::as_u64)
            .and_then(|n| usize::try_from(n).ok())
            .filter(|&n| n <= MAX_FILE_BYTES)
            .ok_or_else(|| {
                decode_err("transfer", "broker returned invalid file snapshot metadata")
            })?;
        let digest = metadata
            .get("sha256")
            .and_then(serde_json::Value::as_str)
            .filter(|s| is_valid_sha256(s))
            .ok_or_else(|| {
                decode_err("transfer", "broker returned invalid file snapshot metadata")
            })?;
        let mut data = Vec::with_capacity(size);
        let mut offset = 0usize;
        while offset < size {
            let part = call_file_broker(
                false,
                &serde_json::json!({
                    "path": path,
                    "action": "read-chunk",
                    "id": revision,
                    "offset": offset,
                }),
            )?;
            let got_offset = part
                .get("offset")
                .and_then(serde_json::Value::as_u64)
                .and_then(|n| usize::try_from(n).ok());
            let encoded = part
                .get("content_base64")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            if got_offset != Some(offset) || encoded.is_empty() {
                return Err(decode_err("transfer", "broker returned an invalid file chunk"));
            }
            let chunk = base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .map_err(|e| decode_err("transfer", e.to_string()))?;
            let count = CHUNK_BYTES.min(size - offset);
            if chunk.len() != count {
                return Err(decode_err("transfer", "broker returned a partial file chunk"));
            }
            data.extend_from_slice(&chunk);
            offset += count;
        }
        if sha256_hex(&data) != digest {
            return Err(decode_err(
                "transfer",
                "broker returned a file snapshot with the wrong digest",
            ));
        }
        Ok(FileSnapshot {
            content: data,
            revision: revision.clone(),
        })
    })();
    if result.is_err() {
        abort_transfer(path, &revision);
    }
    result
}

/// Replace the exact existing file read earlier in this invocation.
pub fn replace_file(path: &str, revision: &str, content: &[u8]) -> Result<(), BridgeError> {
    validate_path(path)?;
    if !is_valid_token(revision) {
        return Err(decode_err(
            "transfer",
            "brokered file requires its opaque read revision",
        ));
    }
    if content.len() > MAX_FILE_BYTES {
        return Err(decode_err(
            "transfer",
            "replacement must be bytes and at most 32 MiB",
        ));
    }
    let started = call_file_broker(
        false,
        &serde_json::json!({
            "path": path,
            "action": "write-start",
            "revision": revision,
            "size": content.len(),
            "sha256": sha256_hex(content),
        }),
    )?;
    let transfer = extract_token(&started, "id")?;
    let mut committing = false;
    let result = (|| {
        let mut offset = 0usize;
        while offset < content.len() {
            let end = (offset + CHUNK_BYTES).min(content.len());
            let part = &content[offset..end];
            let accepted = call_file_broker(
                false,
                &serde_json::json!({
                    "path": path,
                    "action": "write-chunk",
                    "id": transfer,
                    "offset": offset,
                    "content_base64": base64::engine::general_purpose::STANDARD.encode(part),
                }),
            )?;
            let received = accepted
                .get("offset")
                .and_then(serde_json::Value::as_u64)
                .and_then(|n| usize::try_from(n).ok());
            if received != Some(offset + part.len()) {
                return Err(decode_err(
                    "transfer",
                    "broker did not acknowledge the full file chunk",
                ));
            }
            offset = end;
        }
        committing = true;
        let commit_res = call_file_broker(
            true,
            &serde_json::json!({
                "path": path,
                "id": transfer,
            }),
        )?;
        let committed = commit_res
            .get("committed")
            .and_then(serde_json::Value::as_bool)
            == Some(true);
        let has_changed = commit_res
            .get("changed")
            .and_then(serde_json::Value::as_bool)
            .is_some();
        if !committed || !has_changed {
            return Err(decode_err(
                "commit",
                "broker returned no conclusive file commit result",
            ));
        }
        Ok(())
    })();
    if !committing && result.is_err() {
        abort_transfer(path, &transfer);
    }
    result
}
