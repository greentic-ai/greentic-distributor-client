//! Pushing artifacts to an OCI registry.
//!
//! The mirror of `oci_packs`' fetch path: whatever this module stamps on a
//! layer must be a media type that path accepts, or a pushed artifact becomes
//! unfetchable. `media_type_is_always_acceptable_to_the_fetcher` is what keeps
//! the two halves honest.

use crate::oci_packs::{PACK_LAYER_MEDIA_TYPE_OCTET_STREAM, PACK_LAYER_MEDIA_TYPE_ZIP};

/// A pushed artifact and the digest of its CONTENT.
///
/// The digest is deliberately the content digest, not the OCI manifest digest:
/// it is what the deployer's manifest pins as `bundle_digest`, and what the
/// fetch side recomputes from the bytes it downloads. `oci-distribution`'s
/// `PushResponse` carries neither — only `config_url` and `manifest_url` — so
/// this is computed locally from the same bytes that were pushed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PushedPack {
    /// Fully-qualified reference the artifact was pushed to.
    pub reference: String,
    /// `sha256:<hex>` over the artifact bytes.
    pub digest: String,
}

/// Choose the layer media type for an artifact's bytes.
///
/// Only ever returns a member of `oci_packs::default_pack_layer_media_types()`
/// — the fetch path rejects anything else, and a pushed artifact that cannot be
/// fetched is worse than a failed push.
///
/// Detection is by magic bytes rather than by file extension, matching
/// `detect_bundle_archive_kind` in `greentic-start`, which prefers magic over
/// suffix. Anything unrecognised is honestly labelled `application/octet-stream`
/// rather than guessed at; the consumer sniffs magic bytes anyway.
pub fn layer_media_type_for(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(b"PK\x03\x04") {
        return PACK_LAYER_MEDIA_TYPE_ZIP;
    }
    PACK_LAYER_MEDIA_TYPE_OCTET_STREAM
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oci_packs::default_pack_layer_media_types;

    /// Minimal ZIP header — `detect_bundle_archive_kind` in greentic-start
    /// keys off exactly these magic bytes.
    const ZIP_MAGIC: &[u8] = b"PK\x03\x04rest-of-the-archive";
    const UNRECOGNISED: &[u8] = b"\x00\x01\x02\x03 not a known archive";

    #[test]
    fn a_zip_archive_is_stamped_as_a_gtpack_zip() {
        assert_eq!(
            layer_media_type_for(ZIP_MAGIC),
            crate::oci_packs::PACK_LAYER_MEDIA_TYPE_ZIP
        );
    }

    #[test]
    fn an_unrecognised_archive_falls_back_to_octet_stream() {
        assert_eq!(
            layer_media_type_for(UNRECOGNISED),
            crate::oci_packs::PACK_LAYER_MEDIA_TYPE_OCTET_STREAM
        );
    }

    #[test]
    fn an_empty_artifact_still_yields_an_acceptable_media_type() {
        assert_eq!(
            layer_media_type_for(&[]),
            crate::oci_packs::PACK_LAYER_MEDIA_TYPE_OCTET_STREAM
        );
    }

    /// The push/pull contract. If this fails, pushed artifacts are unfetchable
    /// by the very fetcher `greentic-start` uses at container boot.
    #[test]
    fn media_type_is_always_acceptable_to_the_fetcher() {
        let accepted = default_pack_layer_media_types();
        for sample in [ZIP_MAGIC, UNRECOGNISED, &[][..]] {
            let stamped = layer_media_type_for(sample).to_string();
            assert!(
                accepted.contains(&stamped),
                "push stamps {stamped}, which the fetcher does not accept"
            );
        }
    }

    #[test]
    fn pushed_pack_carries_the_content_digest_not_the_manifest_digest() {
        // sha256 of the empty input, the one digest that is trivially checkable.
        let pack = PushedPack {
            reference: "example.test/repo/thing:abc".to_string(),
            digest: crate::oci_packs::compute_digest(&[]),
        };
        assert_eq!(
            pack.digest,
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
