use clap::Parser;

#[derive(Parser)]
struct Args {
}

pub fn main() -> Result<(), i8> {
    let args = Args::parse();

    Ok(())
}
