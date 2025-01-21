use iced_aw::date_picker::Date;

// src/app/state.rs
use crate::types::enums::ExpiryOption;

/// DatePicker params for the broadcast view publish form.
#[derive(Default, Debug, Clone)]
pub struct ExpiryDatePickerParams {
    pub date: Date,
    pub show_picker: bool,
}

#[derive(Debug, Clone)]
pub struct BroadcastFormState {
    pub reward: String,
    pub requirements: String,
    pub description: String,
}
