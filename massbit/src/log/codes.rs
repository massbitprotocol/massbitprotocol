use std::fmt::{Display, Error, Formatter};

pub enum LogCode {
    IndexerStartFailure,
    IndexerSyncingFailure,
    IndexerSyncingFailureNotRecorded,
    BlockIngestionStatus,
    BlockIngestionLagging,
    GraphQlQuerySuccess,
    GraphQlQueryFailure,
    TokioContention,
}

impl Display for LogCode {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let value = match self {
            LogCode::IndexerStartFailure => "IndexerStartFailure",
            LogCode::IndexerSyncingFailure => "IndexerSyncingFailure",
            LogCode::IndexerSyncingFailureNotRecorded => "IndexerSyncingFailureNotRecorded",
            LogCode::BlockIngestionStatus => "BlockIngestionStatus",
            LogCode::BlockIngestionLagging => "BlockIngestionLagging",
            LogCode::GraphQlQuerySuccess => "GraphQLQuerySuccess",
            LogCode::GraphQlQueryFailure => "GraphQLQueryFailure",
            LogCode::TokioContention => "TokioContention",
        };
        write!(f, "{}", value)
    }
}

impl_slog_value!(LogCode, "{}");
