use std::io::Result;
fn main() -> Result<()> {
    prost_build::compile_protos(&["proto/table.proto"], &["proto/"])?;
    Ok(())
}
