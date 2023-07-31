use clap::Parser;

/// CLI application that search duplicate files in a folder
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct CmdArgs {
    /// Root folder to search duplicate
    #[arg(short, long, default_value_t = String::from("/Users/kasunranasinghe/Development/RUST/test"))]
    pub folder: String,
}

#[cfg(test)]
mod tests {}
