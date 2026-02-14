use tracing_subscriber::FmtSubscriber;
use tracing::Level;

pub fn init(verbose: bool) {
    let level = if verbose { Level::INFO } else { Level::ERROR };
    
    // Log only to STDERR to keep STDOUT clean for output.
    // Verbose mode enables INFO logs.
    
    if !verbose {
        return; 
    }

    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_writer(std::io::stderr)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");
}
