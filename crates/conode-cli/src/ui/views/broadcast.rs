use conode_types::work::WorkBroadcast;
use iced::widget::{scrollable, PickList, TextInput};
use iced::{
    alignment::{Horizontal, Vertical},
    theme::{self},
    widget::{Button, Column, Container, Row, Text},
    Alignment, Color, Element, Length,
};
use iced::{Renderer};
use iced_core::Theme;
use iced_aw::DatePicker;
use iced_aw::{date_picker::Date, helpers::date_picker};

// src/ui/views/broadcast.rs

use crate::app::messages::Message;
use crate::app::state::BroadcastFormState;
use crate::types::enums::ExpiryOption;
use crate::ui::styles::common::SectionStyle;
use crate::ui::styles::component::WorkItemStyle;
use crate::ui::styles::container::ContentContainerStyle;
use crate::GUIState;
use crate::ui::func::gui::traits::create::CreateComponent;

pub trait BroadcastView  {
    fn broadcast_form(&self) -> &BroadcastFormState;
    fn broadcast_form_view(&self) -> Element<Message>
    where
        Self: Sized;
    fn network_task_view(&self) -> Element<Message> where Self: Sized;
    fn create_work_item_view<'a>(&self, work_broadcast: &'a WorkBroadcast) -> Element<'a, Message>;
    fn get_work_action(&self, work_broadcast: WorkBroadcast)  -> Option<(String, Message)>;
}

impl BroadcastView for GUIState {
    fn get_work_action(&self, _: WorkBroadcast)  -> Option<(String, Message)> {
        Some(("".to_string(), Message::Noop))
    }

    fn create_work_item_view<'a>(&self, work_broadcast: &'a WorkBroadcast) -> Element<'a, Message> {
        let status_color = match work_broadcast.work.details.status.as_str() {
            "open" | "OPEN" => Color::from_rgb(0.0, 0.8, 0.0),      // Green for open
            "in_progress" | "IN_PROGRESS" => Color::from_rgb(0.0, 0.8, 1.0), // Blue for in progress
            "completed" | "COMPLETED" => Color::from_rgb(0.5, 0.8, 0.5),  // Light green for completed
            "expired" | "EXPIRED" => Color::from_rgb(1.0, 0.0, 0.0),      // Red for expired
            _ => Color::from_rgb(0.5, 0.5, 0.5),                    // Grey for unknown
        };
    
        let mut content = Column::new()
            .push(
                Row::new()
                    .push(Text::new("Work ID:").size(14).style(Color::WHITE))
                    .push(
                        Text::new(&work_broadcast.work.id)
                            .size(14)
                            .style(Color::from_rgb(0.8, 0.8, 0.8)),
                    )
                    .spacing(10),
            )
            .push(
                Row::new()
                    .push(Text::new("Description:").size(14).style(Color::WHITE))
                    .push(
                        Text::new(&work_broadcast.work.details.description)
                            .size(14)
                            .style(Color::from_rgb(0.8, 0.8, 0.8)),
                    )
                    .spacing(10),
            )
            .push(
                Row::new()
                    .push(Text::new("Status:").size(14).style(Color::WHITE))
                    .push(
                        Text::new(&work_broadcast.work.details.status)
                            .size(14)
                            .style(status_color),
                    )
                    .spacing(10),
            );
    
        if let Some(reward) = work_broadcast.work.details.reward {
            content = content.push(
                Row::new()
                    .push(Text::new("Reward:").size(14).style(Color::WHITE))
                    .push(
                        Text::new(reward.to_string())
                            .size(14)
                            .style(Color::from_rgb(0.8, 0.8, 0.8)),
                    )
                    .spacing(10),
            );
        }
    
        if let Some(expiry) = &work_broadcast.work.details.expiry_date {
            content = content.push(
                Row::new()
                    .push(Text::new("Expires:").size(14).style(Color::WHITE))
                    .push(
                        Text::new(expiry.to_rfc2822())
                            .size(14)
                            .style(Color::from_rgb(0.8, 0.8, 0.8)),
                    )
                    .spacing(10),
            );
        }
    
        if !work_broadcast.work.details.requirements.is_empty() {
            content = content.push(
                Row::new()
                    .push(Text::new("Requirements:").size(14).style(Color::WHITE))
                    .push(
                        Text::new(work_broadcast.work.details.requirements.join(", "))
                            .size(14)
                            .style(Color::from_rgb(0.8, 0.8, 0.8)),
                    )
                    .spacing(10),
            );
        }
    
        if let Some(employer) = &work_broadcast.work.employer_address {
            content = content.push(
                Row::new()
                    .push(Text::new("Employer:").size(14).style(Color::WHITE))
                    .push(
                        Text::new(employer)
                            .size(14)
                            .style(Color::from_rgb(0.8, 0.8, 0.8)),
                    )
                    .spacing(10),
            );
        }
    
        if let Some((button_text, message)) = self.get_work_action(work_broadcast.clone()) {
            content = content.push(
                Row::new()
                    .push(iced::widget::Space::with_width(Length::Fill))
                    .push(
                        Button::new(Text::new(button_text).size(12).style(Color::WHITE))
                            .style(theme::Button::Custom(Box::new(
                                crate::ui::styles::button::ActionButtonStyle,
                            )))
                            .padding([6, 12])
                            .on_press(message),
                    ),
            );
        }
    
        Container::new(content)
            .style(theme::Container::Custom(Box::new(WorkItemStyle))) 
            .padding(10)
            .width(Length::Fill)
            .into()
    }
    
    fn network_task_view(&self) -> Element<Message> where Self: Sized {
        let content = Container::new(scrollable(Column::new().spacing(10))).height(Length::Fill).style(theme::Container::Custom(Box::new(ContentContainerStyle)));
        self.create_centered_container(content.into())
    }

    fn broadcast_form(&self) -> &BroadcastFormState {
        &self.broadcast_form
    }

    fn broadcast_form_view(&self) -> Element<Message> where Self: Sized {
        // Modern styling constants
        let input_height = 18;
        let label_color = Color::from_rgb(0.7, 0.7, 0.7);
        
        let title = Column::new()
            .spacing(10)
            .push(
                Text::new("Broadcast Work")
                    .size(24)
                    .style(Color::WHITE)
            )
            .push(
                Text::new("Create a new work opportunity with the details below")
                    .size(14)
                    .style(label_color)
            );
    

        let description_input = Column::new()
            .spacing(5)
            .push(Text::new("Description").size(14).style(label_color))
            .push(
                TextInput::new(
                    "Enter detailed description",
                    &self.broadcast_form().description,
                )
                .padding(12)
                .size(input_height)
                
                .width(Length::Fill)
                
                .on_input(Message::DescriptionChanged)
            );
    
        let requirements_row = Row::new()
            .spacing(20)
            .width(Length::Fill)
            .push(
                Column::new()
                    .width(Length::FillPortion(2))
                    .spacing(5)
                    .push(Text::new("Requirements").size(14).style(label_color))
                    .push(
                        TextInput::new(
                            "Enter requirements (comma separated)",
                            &self.broadcast_form().requirements,
                        )
                        .padding(10)
                        .size(input_height)
                        .width(Length::Fill)
                        .on_input(Message::RequirementsChanged)
                    )
            );


        let payment_row = Row::new()
            .spacing(20)
            .width(Length::Fill)
            .push(
                Column::new()
                    .width(Length::Fixed(90.0))
                    .spacing(5)
                    .push(Text::new("Reward").size(14).style(label_color))
                    .push(
                        TextInput::new(
                            "$0.00",
                            &self.broadcast_form().reward
                        )
                        .padding(10)
                        .size(input_height)
                        .width(Length::Fill)
                        .on_input(Message::RewardChanged)
                    )
            );
    
        let publish_button = Button::new(
            Text::new("Publish to Network")
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center)
        )
        .width(Length::FillPortion(1))
        .padding([10, 20])
        .style(theme::Button::Primary)
        .on_press(Message::BroadcastFormSubmit);
    
        // Main form layout
        let form_content = Column::new()
            .spacing(24)
            .width(Length::Fill)
            .max_width(800)
            .push(title)
            .push(description_input)
            .push(requirements_row)
            .push(payment_row)
            .push(
                Row::new()
                    .spacing(20)
                    .push(publish_button)
            );
    
        // Wrap in container for padding and centering
        Container::new(form_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(40)
            .center_x()
            .into()
    }

}