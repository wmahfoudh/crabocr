use thiserror::Error;

#[derive(Error, Debug)]
pub enum CrabError {
    #[error("CLI Usage Error: {0}")]
    Cli(String), // Exit 1
    
    #[error("Input Read Error: {0}")]
    Input(#[from] std::io::Error), // Exit 2
    
    #[error("PDF Error: {0}")]
    Pdf(String), // Exit 3
    
    #[error("OCR Error: {0}")]
    Ocr(String), // Exit 4
    
    #[error("Internal Error: {0}")]
    Internal(String), // Exit 5
}

impl CrabError {
    pub fn exit_code(&self) -> i32 {
        match self {
            CrabError::Cli(_) => 1,
            CrabError::Input(_) => 2,
            CrabError::Pdf(_) => 3,
            CrabError::Ocr(_) => 4,
            CrabError::Internal(_) => 5,
        }
    }
}
