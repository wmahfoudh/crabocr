use tracing_subscriber::FmtSubscriber;
use tracing::Level;

pub fn init(verbose: bool) {
    let level = if verbose { Level::INFO } else { Level::ERROR };
    
    // We only want to log to stderr.
    // In verbose mode, we log INFO and above.
    // In strict mode, we might not want to log anything unless it's a fatal error?
    // The spec says "Output: STYLIZED LOGS ONLY TO STDERR. STDOUT IS FOR TEXT ONLY".
    // "No logs without --verbose".
    
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
