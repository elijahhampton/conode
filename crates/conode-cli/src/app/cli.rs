// src/app/cli.rs

use crate::types::enums::View;
use crate::ui::func::gui::traits::create::CreateComponent;
use crate::ui::styles::component::ProposalItemStyle;
use crate::ui::views::active_work::ActiveWorkView;
use crate::ui::views::logs::LogsView;
use crate::ui::views::main::MainView;
use crate::ui::views::options::OptionsView;
use crate::ui::views::proposals::ProposalView;
use crate::ui::views::restore_seed::RestoreSeedView;

use crate::ui::views::work::WorkView;
use conode_types::negotiation::Negotiation;
use conode_types::work::{ActiveWork, JobRole, WorkBroadcast};
use futures::future::BoxFuture;
use iced::widget::{Space, TextInput};
use iced::{
    alignment::{Horizontal, Vertical},
    theme::{self},
    widget::{Button, Column, Container, Row, Scrollable, Text},
    Alignment, Color, Element, Length,
};
use libp2p::PeerId;

use crate::ui::styles::button::{ActionButtonStyle, OutlinedButtonStyle};
use crate::{ui::views::mnemonic::MnemonicView, GUIState};

use super::messages::Message;
use super::state::BroadcastFormState;

use crate::ui::views::broadcast::BroadcastView;
