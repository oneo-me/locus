use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::time::{Duration, Instant};

use base64::Engine;
use locus_library::{ContentTarget, LibraryAccess};
use serde_json::{Value, json};

const MAX_UPLOAD_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_CHUNK_BYTES: usize = 256 * 1024;
const TRANSFER_LIFETIME: Duration = Duration::from_secs(600);

struct Upload {
    target: ContentTarget,
    expected: String,
    size: u64,
    received: u64,
    sha256: Option<String>,
    file: File,
    touched: Instant,
}

#[derive(Default)]
pub(super) struct Transfers {
    uploads: HashMap<String, Upload>,
    next_id: u64,
}

impl Transfers {
    pub(super) fn expire(&mut self) {
        self.uploads
            .retain(|_, upload| upload.touched.elapsed() < TRANSFER_LIFETIME);
    }

    pub(super) fn begin(
        &mut self,
        library: &LibraryAccess,
        target: ContentTarget,
        size: u64,
        sha256: Option<String>,
        expected: String,
    ) -> Result<Value, String> {
        if size > MAX_UPLOAD_BYTES
            || self.uploads.len() >= 4
            || self.uploads.values().map(|u| u.size).sum::<u64>() + size > MAX_UPLOAD_BYTES
        {
            return Err("upload_limit: At most four uploads and 1 GiB total per session.".into());
        }
        if sha256
            .as_ref()
            .is_some_and(|value| value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            return Err("invalid_checksum: Expected a hexadecimal SHA-256 digest.".into());
        }
        library.validate_write(&target, &expected)?;
        let file = tempfile::tempfile()
            .map_err(|_| "transfer_io_error: Cannot stage upload.".to_owned())?;
        self.next_id += 1;
        let id = self.next_id.to_string();
        self.uploads.insert(
            id.clone(),
            Upload {
                target,
                expected,
                size,
                received: 0,
                sha256,
                file,
                touched: Instant::now(),
            },
        );
        Ok(
            json!({"uploadId": id, "chunkSize": MAX_CHUNK_BYTES, "expiresAfterIdleSeconds": TRANSFER_LIFETIME.as_secs()}),
        )
    }

    pub(super) fn append(&mut self, id: &str, offset: u64, encoded: &str) -> Result<Value, String> {
        if encoded.len() > MAX_CHUNK_BYTES.div_ceil(3) * 4 {
            return Err("chunk_too_large: Maximum decoded chunk size is 256 KiB.".into());
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|_| "invalid_base64: Chunk could not be decoded.".to_owned())?;
        if bytes.is_empty() || bytes.len() > MAX_CHUNK_BYTES {
            return Err("invalid_chunk: Supply 1..262144 bytes.".into());
        }
        let upload = self
            .uploads
            .get_mut(id)
            .ok_or("unknown_upload: Upload expired or does not exist in this session.")?;
        let end = offset
            .checked_add(bytes.len() as u64)
            .ok_or("invalid_offset")?;
        if end > upload.size || offset > upload.received {
            return Err("invalid_offset: Append at received offset within declared size.".into());
        }
        upload
            .file
            .seek(SeekFrom::Start(offset))
            .map_err(transfer_error)?;
        if offset < upload.received {
            if end > upload.received {
                return Err("invalid_offset: Retry must match an already received chunk.".into());
            }
            let mut previous = vec![0; bytes.len()];
            upload
                .file
                .read_exact(&mut previous)
                .map_err(transfer_error)?;
            if previous != bytes {
                return Err("chunk_conflict: Retry differs from received bytes.".into());
            }
        } else {
            upload.file.write_all(&bytes).map_err(transfer_error)?;
            upload.received = end;
        }
        upload.touched = Instant::now();
        Ok(json!({"uploadId": id, "received": upload.received, "size": upload.size}))
    }

    pub(super) fn finish(&mut self, library: &LibraryAccess, id: &str) -> Result<Value, String> {
        let upload = self
            .uploads
            .get_mut(id)
            .ok_or("unknown_upload: Upload expired or does not exist in this session.")?;
        if upload.received != upload.size {
            return Err("incomplete_upload: Supply all declared bytes before committing.".into());
        }
        upload
            .file
            .seek(SeekFrom::Start(0))
            .map_err(transfer_error)?;
        let (_, checksum) =
            locus_library::content_version_from_reader(&mut upload.file).map_err(transfer_error)?;
        if upload
            .sha256
            .as_ref()
            .is_some_and(|expected| !expected.eq_ignore_ascii_case(&checksum))
        {
            return Err("checksum_mismatch: Uploaded bytes do not match SHA-256.".into());
        }
        upload
            .file
            .seek(SeekFrom::Start(0))
            .map_err(transfer_error)?;
        let mut result =
            library.write_content(&upload.target, &mut upload.file, &upload.expected)?;
        result["uploadedSha256"] = json!(checksum);
        self.uploads.remove(id);
        Ok(result)
    }

    pub(super) fn cancel(&mut self, id: &str) -> Result<Value, String> {
        let removed = self.uploads.remove(id).is_some();
        Ok(json!({"uploadId": id, "cancelled": removed}))
    }
}

fn transfer_error(_: std::io::Error) -> String {
    "transfer_io_error: Unable to read or write staged content.".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use locus_library::Resource;

    #[test]
    fn failed_and_cancelled_uploads_never_change_destination() {
        let root = tempfile::tempdir().unwrap();
        let library = LibraryAccess::new(Some(root.path().to_owned()));
        library.create_note("locus://note", "Note").unwrap();
        let target = ContentTarget::new(
            "locus://note",
            Resource::Attachment {
                name: "data.bin".into(),
            },
        );
        let mut transfers = Transfers::default();
        let started = transfers
            .begin(
                &library,
                target.clone(),
                3,
                Some("0".repeat(64)),
                "missing".into(),
            )
            .unwrap();
        let id = started["uploadId"].as_str().unwrap();
        assert!(transfers.append(id, 1, "YWJj").is_err());
        transfers.append(id, 0, "YWJj").unwrap();
        assert!(transfers.append(id, 0, "eHl6").is_err());
        assert!(
            transfers
                .finish(&library, id)
                .unwrap_err()
                .starts_with("checksum_mismatch")
        );
        assert!(!root.path().join("note/data.bin").exists());
        transfers.cancel(id).unwrap();
        assert!(transfers.finish(&library, id).is_err());
        let started = transfers
            .begin(&library, target.clone(), 3, None, "missing".into())
            .unwrap();
        let id = started["uploadId"].as_str().unwrap();
        transfers.append(id, 0, "YWJj").unwrap();
        library.write_text(&target, "external", "missing").unwrap();
        assert!(
            transfers
                .finish(&library, id)
                .unwrap_err()
                .starts_with("conflict")
        );
        assert_eq!(library.read_text(&target).unwrap().0, "external");
    }

    #[test]
    fn expired_uploads_are_removed_and_empty_uploads_commit() {
        let root = tempfile::tempdir().unwrap();
        let library = LibraryAccess::new(Some(root.path().to_owned()));
        library.create_note("locus://note", "Note").unwrap();
        let target = ContentTarget::new(
            "locus://note",
            Resource::Attachment {
                name: "empty.txt".into(),
            },
        );
        let mut transfers = Transfers::default();
        let started = transfers
            .begin(&library, target.clone(), 0, None, "missing".into())
            .unwrap();
        let id = started["uploadId"].as_str().unwrap();
        transfers.uploads.get_mut(id).unwrap().touched = Instant::now() - TRANSFER_LIFETIME;
        transfers.expire();
        assert!(transfers.finish(&library, id).is_err());
        let started = transfers
            .begin(&library, target, 0, None, "missing".into())
            .unwrap();
        let finished = transfers
            .finish(&library, started["uploadId"].as_str().unwrap())
            .unwrap();
        assert_eq!(finished["size"], 0);
    }
}
