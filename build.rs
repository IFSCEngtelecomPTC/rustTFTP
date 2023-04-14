use std::io::Result;
extern crate prost_build;

fn main() -> Result<()> {
    prost_build::compile_protos(&["src/tftp2.proto"], &["src/"])?;
    Ok(())
}