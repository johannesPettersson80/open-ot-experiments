use std::error::Error;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vectors");
    let written = open_ot_carriage::vectors::write_vectors(&root)?;
    println!("wrote {written} vector files to {}", root.display());
    Ok(())
}
