use std::fs;

use anyhow::Result;

// prost 只能编译proto的message，还不能编译service
// 需要使用tonic，tonic-build编译service

fn main() -> Result<()> {
    let path = "src/pb";
    // Recursively create a directory and all of its parent components if they are missing
    fs::create_dir_all(path)?;
    let builder = tonic_build::configure();
    builder.out_dir("src/pb").compile(
        &[
            "../protos/notification/messages.proto",
            "../protos/notification/rpc.proto",
        ],
        &["../protos"],
    )?;
    Ok(())
}
