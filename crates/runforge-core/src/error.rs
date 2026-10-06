/// A file the bench cannot open. The window shows this text and stays empty.
#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    #[error("no run_history.json in this folder")]
    NotFound,
    #[error("could not read run_history.json: {source}")]
    Read {
        #[source]
        source: std::io::Error,
    },
    #[error("run history is not valid JSON: {0}")]
    Parse(String),
    #[error("run history must be a JSON array")]
    NotArray,
    #[error("no run-config series in this folder")]
    NoSeries,
}
