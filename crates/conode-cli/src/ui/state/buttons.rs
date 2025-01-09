use std::default;

use iced::mouse::Button;

use crate::{app::Message, ui::views::main::MainContentView};

#[derive(Default)]
pub struct ButtonState {
    pub title: String,
    pub action: Message,
    pub disabled: bool
} 

impl ButtonState {
    pub fn new(title: String, action: Message, disabled: bool) -> Self {
        Self {
            title,
            action,
            disabled
        }
    }

    pub fn set_title(&mut self, new_title: String) {
        self.title = new_title;
    }
}
#[derive(Default)]
pub struct ToolbarButtonsState {
    pub connect: ButtonState,
    pub discover: ButtonState,
    pub publish: ButtonState,
    pub view_proposals: ButtonState,
    pub view_task: ButtonState,
    pub details: ButtonState
}

impl ToolbarButtonsState {
    pub fn default() -> Self {
        Self {
            connect: ButtonState::new("Start Working".to_string(), Message::StartNode, false),
            discover: ButtonState::new("Discover Task".to_string(), Message::SwitchMainView(MainContentView::DiscoverTask), true),
            publish: ButtonState::new("Publish Task".to_string(), Message::SwitchMainView(MainContentView::PublishTask), true),
            view_proposals: ButtonState::new("View Proposals".to_string(),  Message::SwitchMainView(MainContentView::ViewProposals), true),
            view_task: ButtonState::new("View Task".to_string(), Message::SwitchMainView(MainContentView::ViewTasks), true),
            details: ButtonState::new("Node Details".to_string(), Message::SwitchMainView(MainContentView::ViewDetails), true)
        }
    }

    pub fn enable_one(&self) {}

    pub fn enable_all(&self) {}

    pub fn disable_one(&self) {}

    pub fn disable_all(&self) {}
}   
