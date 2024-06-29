use std::fs;

use anyhow::Result;
use proto_builder_trait::tonic::BuilderAttributes;

// prost 只能编译proto的message，还不能编译service
// 需要使用tonic，tonic-build编译service

fn main() -> Result<()> {
    let path = "src/pb";
    // Recursively create a directory and all of its parent components if they are missing
    fs::create_dir_all(path)?;
    let builder = tonic_build::configure();
    builder
        .out_dir("src/pb")
        .with_serde(
            &["User"],
            true,
            true,
            Some(&[r#"#[serde(rename_all = "camelCase")]"#]),
        )
        .with_sqlx_from_row(&["User"], None)
        .with_derive_builder(
            &[
                "User",
                "QueryRequest",
                "RawQueryRequest",
                "TimeQuery",
                "IdQuery",
            ],
            None,
        )
        .with_field_attributes(
            &["User.email", "User.name", "RawQueryRequest.query"],
            &[r#"#[builder(setter(into))]"#],
        )
        .with_field_attributes(
            &["TimeQuery.before", "TimeQuery.after"],
            &[r#"#[builder(setter(into, strip_option))]"#],
        )
        .with_field_attributes(
            &["QueryRequest.timestamps"],
            &[r#"#[builder(setter(each(name="timestamp", into)))]"#],
        )
        .with_field_attributes(
            &["QueryRequest.ids"],
            &[r#"#[builder(setter(each(name="id", into)))]"#],
        )
        .compile(
            &[
                "../protos/user-stats/messages.proto",
                "../protos/user-stats/rpc.proto",
            ],
            &["../protos"],
        )?;
    Ok(())
}
