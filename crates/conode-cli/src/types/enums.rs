// src/types/enums.rs
#[derive(Clone, Debug)]
pub enum View {
    Main,
    Mnemonic,
    RestoreSeed,
    Options,
    Logs,
    BroadcastForm,
    WorkOpportunities,
    Proposals,
    ActiveWork,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ExpiryOption {
    OneDay,
    ThreeDays,
    OneWeek,
    TwoWeeks,
    OneMonth,
}

impl std::fmt::Display for ExpiryOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExpiryOption::OneDay => write!(f, "1 Day"),
            ExpiryOption::ThreeDays => write!(f, "3 Days"),
            ExpiryOption::OneWeek => write!(f, "1 Week"),
            ExpiryOption::TwoWeeks => write!(f, "2 Weeks"),
            ExpiryOption::OneMonth => write!(f, "1 Month"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum WorkTab {
    Stored,
    Saved,
}
