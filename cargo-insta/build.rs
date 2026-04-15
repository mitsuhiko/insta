use std::collections::BTreeSet;

use shadow_rs::SdResult;
use shadow_rs::ShadowBuilder;
use shadow_rs::CARGO_METADATA;
use shadow_rs::CARGO_TREE;

fn main() -> SdResult<()> {
    ShadowBuilder::builder()
        // exclude these two large constants that we don't need
        .deny_const(BTreeSet::from([CARGO_METADATA, CARGO_TREE]))
        .build()?;

    Ok(())
}
