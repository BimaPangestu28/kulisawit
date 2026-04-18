use anyhow::Result;

pub fn run() -> Result<()> {
    println!("kulisawit {}", env!("CARGO_PKG_VERSION"));
    Ok(())
}
