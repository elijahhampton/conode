use futures::future::BoxFuture;
use iced::alignment::Horizontal;
use iced::widget::{Button, TextInput};
use iced::{theme, Element};

use crate::app::messages::Message;
use crate::ui::func::gui::traits::create::CreateComponent;
use crate::ui::styles::button::ActionButtonStyle;
use crate::ui::styles::component::ProposalItemStyle;
use crate::GUIState;
use conode_types::work::{ActiveWork, JobRole};
use iced::widget::Scrollable;
use iced::{
    widget::{Column, Container, Row, Space, Text},
    Alignment, Color, Length,
};

pub trait ActiveWorkView {
    fn active_works(&self) -> &Vec<ActiveWork>;

    fn create_active_work_item_view<'a>(
        &'a self,
        active_work: &'a ActiveWork,
    ) -> Element<'a, Message>;
    fn active_work_view(&self) -> Element<Message>;
    fn get_solution(
        &self,
        work_id: String,
    ) -> BoxFuture<'_, Result<Option<String>, Box<dyn std::error::Error + Send + Sync>>>;
    #[allow(dead_code)]
    fn get_work_action(
        &self,
        active_work: ActiveWork,
    ) -> BoxFuture<'_, Result<Option<(String, Message)>, Box<dyn std::error::Error + Send + Sync>>>;
}

impl ActiveWorkView for GUIState {
    fn active_works(&self) -> &Vec<ActiveWork> {
        &self.active_works
    }

    fn get_solution(
        &self,
        work_id: String,
    ) -> BoxFuture<'_, Result<Option<String>, Box<dyn std::error::Error + Send + Sync>>> {
        Box::pin(async move {
            if let Some(node) = &self.node {
                node.lock().await.solution_encrypted(work_id).await
            } else {
                Ok(None)
            }
        })
    }

    fn get_work_action(
        &self,
        work: ActiveWork,
    ) -> BoxFuture<'_, Result<Option<(String, Message)>, Box<dyn std::error::Error + Send + Sync>>>
    {
        Box::pin(async move {
            match work.role {
                JobRole::Worker => {
                    let solution = self.get_solution(work.work.id.clone()).await?;
                    Ok(if solution.is_none() {
                        Some(("Submit Solution".to_string(), Message::SubmitSolution(work)))
                    } else {
                        None
                    })
                }
                JobRole::Employer => {
                    let solution = self.get_solution(work.work.id.clone()).await?;
                    Ok(if solution.is_some() {
                        Some(("Review Solution".to_string(), Message::ReviewSolution(work)))
                    } else {
                        None
                    })
                }
            }
        })
    }

    fn create_active_work_item_view<'a>(
        &'a self, // Add lifetime annotation here
        active_work: &'a ActiveWork,
    ) -> Element<'a, Message> {
        let role_text = match active_work.role {
            JobRole::Employer => "Employer",
            JobRole::Worker => "Worker",
        };

        let mut content = Column::new()
            .push(
                Row::new()
                    .push(Text::new("Work ID:").size(14).style(Color::WHITE))
                    .push(
                        Text::new(&active_work.work.id)
                            .size(14)
                            .style(Color::from_rgb(0.8, 0.8, 0.8)),
                    )
                    .spacing(10),
            )
            .push(
                Row::new()
                    .push(Text::new("Role:").size(14).style(Color::WHITE))
                    .push(
                        Text::new(role_text)
                            .size(14)
                            .style(Color::from_rgb(0.8, 0.8, 0.8)),
                    )
                    .spacing(10),
            )
            .push(
                Row::new()
                    .push(Text::new("Reward:").size(14).style(Color::WHITE))
                    .push(
                        Text::new(active_work.work.reward.to_string())
                            .size(14)
                            .style(Color::from_rgb(0.8, 0.8, 0.8)),
                    )
                    .spacing(10),
            );

        // Add solution input and submit button for worker

        if self
            .active_work_solutions_submitted
            .get(&active_work.work.id.clone())
            .is_some_and(|val| val.is_some())
            && matches!(active_work.role, JobRole::Employer)
        {
            let solution = self
                .active_work_solutions_submitted
                .get(&active_work.work.id.clone())
                .unwrap()
                .as_ref()
                .unwrap();
            let row = Row::new()
                .push(Text::new("Solution:  "))
                .push(Text::new(solution.to_string()));
            content = content.push(row).push(
                Button::new(Text::new("Accept Solution").horizontal_alignment(Horizontal::Center))
                    .on_press(Message::SolutionTestedInternally(
                        active_work.work.id.clone(),
                        *solution,
                    )),
            );
        }

        if matches!(active_work.role, JobRole::Worker) {
            content = content.push(Space::with_height(Length::Fixed(10.0))).push(
                Column::new()
                    .push(
                        TextInput::new(
                            "Enter solution...",
                            &self
                                .active_work_solutions
                                .get(&active_work.work.id)
                                .cloned()
                                .unwrap_or_default(),
                        )
                        .padding(10)
                        .size(14)
                        .on_input(move |s| {
                            Message::SolutionChanged(active_work.work.id.clone(), s)
                        }),
                    )
                    .push(Space::with_height(Length::Fixed(10.0)))
                    .push(
                        Button::new(
                            Text::new("Submit Solution").horizontal_alignment(Horizontal::Center),
                        )
                        .width(Length::Fill)
                        .padding(10)
                        .style(theme::Button::Custom(Box::new(ActionButtonStyle)))
                        .on_press(Message::SubmitSolution(active_work.clone())),
                    ),
            );
        }

        Container::new(content)
            .style(theme::Container::Custom(Box::new(ProposalItemStyle)))
            .padding(10)
            .width(Length::Fill)
            .into()
    }

    fn active_work_view(&self) -> Element<Message> {
        let mut active_works_list = Column::new().spacing(10).padding(10);

        active_works_list = self
            .active_works()
            .iter()
            .fold(active_works_list, |column, work| {
                column.push(self.create_active_work_item_view(work))
            });

        let scrollable_content = Scrollable::new(active_works_list)
            .height(Length::Fill)
            .width(Length::Fill);

        let content = Column::new()
            .push(Text::new("Active Work").size(24).style(Color::WHITE))
            .push(scrollable_content)
            .spacing(20)
            .align_items(Alignment::Center);

        self.create_centered_container(content.into())
    }
}
