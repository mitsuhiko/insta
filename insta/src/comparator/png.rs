//! Provides a [`Comparator`] for comparing binary [`Snapshot`]s that hold
//! compressed PNG data.

struct Comparator;

use crate::env::ToolConfig;
use crate::snapshot::{Snapshot, SnapshotContents};

impl super::Comparator for Comparator {
    fn matches(&self, _config: &ToolConfig, reference: &Snapshot, test: &Snapshot) -> bool {
        let reference: &[u8] = match reference.contents() {
            SnapshotContents::Binary(data) => &data,
            _ => panic!("reference data is not binary; cannot decode PNG"),
        };
        let test: &[u8] = match test.contents() {
            SnapshotContents::Binary(data) => &data,
            _ => panic!("test data is not binary; cannot decode PNG"),
        };
        let decoder = png::Decoder::new(std::io::Cursor::new(reference));
        let mut reader = decoder
            .read_info()
            .expect("cannot decode PNG header from reference data");
        let mut ref_image_data = vec![
            0;
            reader.output_buffer_size().expect(
                "insufficient memory to decode PNG frame from reference data"
            )
        ];
        let ref_info = reader
            .next_frame(&mut ref_image_data)
            .expect("cannot read PNG frame from reference data");
        let ref_image_data = &ref_image_data[0..ref_info.buffer_size()];

        let decoder = png::Decoder::new(std::io::Cursor::new(test));
        reader = decoder
            .read_info()
            .expect("cannot decode PNG header from test data");
        let mut test_image_data = vec![
            0;
            reader.output_buffer_size().expect(
                "insufficient memory to decode PNG frame from test data"
            )
        ];
        let test_info = reader
            .next_frame(&mut test_image_data)
            .expect("cannot read PNG frame from test data");
        let test_image_data = &test_image_data[0..test_info.buffer_size()];

        ref_info == test_info && ref_image_data == test_image_data
    }
}

/// Compares binary [`Snapshot`]s of compressed PNG data. The frame data and
/// metadata for the first frames of each image are checked for equality. Images
/// with different compression levels compare the same. Images with different
/// pixel formats will not.
pub const COMPARATOR: &'static dyn super::Comparator = &Comparator;

#[cfg(test)]
mod test {
    use super::COMPARATOR;

    use crate::env::ToolConfig;
    use crate::snapshot::{
        MetaData, Snapshot, SnapshotContents, TextSnapshotContents, TextSnapshotKind,
    };
    use std::rc::Rc;

    static XEYES_PNG: &'static [u8] = include_bytes!("../../tests/xeyes.png");
    static XEYES_ANGEL_PNG: &'static [u8] = include_bytes!("../../tests/xeyes-angel.png");
    static COMPRESSED_16BPC_RGB_PNG: &'static [u8] = include_bytes!("compressed_16bpc_rgb.png");
    static COMPRESSED_16BPC_RGB_INTERLACED_PNG: &'static [u8] = include_bytes!("compressed_16bpc_rgb_interlaced.png");
    static COMPRESSED_16BPC_RGBA_PNG: &'static [u8] = include_bytes!("compressed_16bpc_rgba.png");
    static COMPRESSED_8BPC_RGB_PNG: &'static [u8] = include_bytes!("compressed_8bpc_rgb.png");
    static UNCOMPRESSED_16BPC_RGB_PNG: &'static [u8] = include_bytes!("uncompressed_16bpc_rgb.png");

    fn snapshot_of(data: &[u8]) -> Snapshot {
        Snapshot::from_components(
            String::from("test"),
            None,
            MetaData::default(),
            SnapshotContents::Binary(Rc::new(data.to_vec())),
        )
    }

    fn text_snapshot() -> Snapshot {
        Snapshot::from_components(
            String::from("test"),
            None,
            MetaData::default(),
            SnapshotContents::Text(TextSnapshotContents::new(
                String::from(
                    "The sky above the port was the color of television, tuned to a dead channel.",
                ),
                TextSnapshotKind::Inline,
            )),
        )
    }

    #[test]
    fn compare_identical_images() {
        assert!(COMPARATOR.matches(
            &ToolConfig::default(),
            &snapshot_of(XEYES_PNG),
            &snapshot_of(XEYES_PNG)
        ));
    }

    #[test]
    fn compare_different_images() {
        assert!(!COMPARATOR.matches(
            &ToolConfig::default(),
            &snapshot_of(XEYES_PNG),
            &snapshot_of(XEYES_ANGEL_PNG)
        ));
    }

    #[test]
    fn images_with_different_compression_levels_match() {
        assert!(COMPARATOR.matches(
            &ToolConfig::default(),
            &snapshot_of(COMPRESSED_16BPC_RGB_PNG),
            &snapshot_of(UNCOMPRESSED_16BPC_RGB_PNG),
        ));
    }

    #[test]
    fn images_with_different_bpp_no_match() {
        assert!(!COMPARATOR.matches(
            &ToolConfig::default(),
            &snapshot_of(COMPRESSED_8BPC_RGB_PNG),
            &snapshot_of(COMPRESSED_16BPC_RGB_PNG),
        ));
    }

    #[test]
    fn images_with_different_pixel_colors_no_match() {
        assert!(!COMPARATOR.matches(
            &ToolConfig::default(),
            &snapshot_of(COMPRESSED_16BPC_RGB_PNG),
            &snapshot_of(COMPRESSED_16BPC_RGBA_PNG),
        ));
    }

    #[test]
    fn images_with_different_interlacing_match() {
        assert!(COMPARATOR.matches(
            &ToolConfig::default(),
            &snapshot_of(COMPRESSED_16BPC_RGB_PNG),
            &snapshot_of(COMPRESSED_16BPC_RGB_INTERLACED_PNG),
        ));
    }

    #[test]
    #[should_panic(expected = "reference data is not binary")]
    fn compare_invalid_reference_snapshot_type() {
        let _ = COMPARATOR.matches(
            &ToolConfig::default(),
            &text_snapshot(),
            &snapshot_of(XEYES_PNG),
        );
    }

    #[test]
    #[should_panic(expected = "test data is not binary")]
    fn compare_invalid_test_snapshot_type() {
        let _ = COMPARATOR.matches(
            &ToolConfig::default(),
            &snapshot_of(XEYES_PNG),
            &text_snapshot(),
        );
    }

    #[test]
    #[should_panic(expected = "cannot decode PNG header from reference data")]
    fn compare_invalid_reference_image() {
        let _ = COMPARATOR.matches(
            &ToolConfig::default(),
            &snapshot_of(&[0xCA, 0xFE, 0xBA, 0xBE]),
            &snapshot_of(XEYES_PNG),
        );
    }

    #[test]
    #[should_panic(expected = "cannot decode PNG header from test data")]
    fn compare_invalid_test_image() {
        let _ = COMPARATOR.matches(
            &ToolConfig::default(),
            &snapshot_of(XEYES_PNG),
            &snapshot_of(&[0xCA, 0xFE, 0xBA, 0xBE]),
        );
    }
}
