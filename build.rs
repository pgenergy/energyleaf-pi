use std::io::Result;
fn main() -> Result<()> {
    prost_build::compile_protos(&["src/proto/energyleaf.proto"], &["src/proto"])?;
    Ok(())
}
