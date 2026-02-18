mod types;
pub use types::*;

mod nsjail;
pub use nsjail::*;

mod overlay;
pub use overlay::*;

#[cfg(feature = "private")]
mod s3_ee;
mod s3_oss;
pub use s3_oss::*;

use std::path::Path;
use windmill_common::error::{self, Error};

/// Unpack a tar.gz byte stream into `dest_path`.
pub fn untar_gz(bytes: &[u8], dest_path: &Path) -> error::Result<()> {
    use flate2::read::GzDecoder;
    use std::io::Cursor;
    use tar::Archive;

    let decoder = GzDecoder::new(Cursor::new(bytes));
    let mut archive = Archive::new(decoder);
    archive
        .unpack(dest_path)
        .map_err(|e| Error::ExecutionErr(format!("Failed to unpack tar.gz: {e}")))?;
    Ok(())
}

/// Tar+gzip a directory into bytes. Symlinks are preserved as-is (not followed).
pub fn tar_gz(source_path: &Path) -> error::Result<Vec<u8>> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use tar::Builder;

    let buf = Vec::new();
    let encoder = GzEncoder::new(buf, Compression::fast());
    let mut builder = Builder::new(encoder);
    builder.follow_symlinks(false);
    builder
        .append_dir_all(".", source_path)
        .map_err(|e| Error::ExecutionErr(format!("Failed to tar directory: {e}")))?;
    let encoder = builder
        .into_inner()
        .map_err(|e| Error::ExecutionErr(format!("Failed to finish tar: {e}")))?;
    encoder
        .finish()
        .map_err(|e| Error::ExecutionErr(format!("Failed to finish gzip: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tar_gz_roundtrip() {
        let source_dir = tempfile::tempdir().unwrap();
        std::fs::write(source_dir.path().join("hello.txt"), "world").unwrap();
        std::fs::create_dir(source_dir.path().join("subdir")).unwrap();
        std::fs::write(source_dir.path().join("subdir/nested.txt"), "nested content").unwrap();

        let bytes = tar_gz(source_dir.path()).unwrap();
        assert!(!bytes.is_empty());

        let dest_dir = tempfile::tempdir().unwrap();
        untar_gz(&bytes, dest_dir.path()).unwrap();

        assert_eq!(
            std::fs::read_to_string(dest_dir.path().join("hello.txt")).unwrap(),
            "world"
        );
        assert_eq!(
            std::fs::read_to_string(dest_dir.path().join("subdir/nested.txt")).unwrap(),
            "nested content"
        );
    }

    #[test]
    fn test_tar_gz_empty_dir() {
        let source_dir = tempfile::tempdir().unwrap();
        let bytes = tar_gz(source_dir.path()).unwrap();
        assert!(!bytes.is_empty());

        let dest_dir = tempfile::tempdir().unwrap();
        untar_gz(&bytes, dest_dir.path()).unwrap();
    }
}
