use futures::future::BoxFuture;
use iced::widget::Button;
use iced::Element;

use crate::app::messages::Message;
use conode_types::work::ActiveWork;

pub trait ActiveWorkView {
    fn active_works(&self) -> &Vec<ActiveWork>;
    fn create_centered_container<'a>(&self, content: Element<'a, Message>) -> Element<'a, Message>;
    fn create_button<'a>(&self, label: &'a str, message: Message) -> Button<'a, Message>;
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
