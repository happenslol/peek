use iced::{Element, Subscription, Task, widget::horizontal_space, window::Id};
use tracing::warn;
use wayland_client::protocol::wl_output::WlOutput;

use crate::{
  border::{self, Border},
  clock::{self, Clock},
};

#[derive(Debug, Clone)]
pub struct Output {
  border_top: Border,
  border_bottom: Border,
  clock: Option<Clock>,
  wl_output: WlOutput,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
  Ignore,
  ShowClock,
  Clock(clock::Message),
  MouseEvent(iced::mouse::Event, Id),
  Border(border::Position, border::Message),
}

impl Output {
  pub fn new(output: &WlOutput) -> (Self, Task<Message>) {
    let (border_top, top_task) = Border::new(output, border::Position::Top);
    let (border_bottom, bottom_task) = Border::new(output, border::Position::Bottom);

    let result = Output {
      border_top,
      border_bottom,
      clock: None,
      wl_output: output.clone(),
    };

    (
      result,
      Task::batch(vec![
        top_task.map(|_| Message::Ignore),
        bottom_task.map(|_| Message::Ignore),
      ]),
    )
  }

  pub fn has(&self, id: Id) -> bool {
    self.border_top.has(id)
      || self.border_bottom.has(id)
      || self.clock.as_ref().is_some_and(|c| c.has(id))
  }

  pub fn subscription(&self) -> Option<Subscription<Message>> {
    self
      .clock
      .as_ref()
      .map(|c| c.subscription().map(Message::Clock))
  }

  pub fn update(&mut self, message: Message) -> Task<Message> {
    match message {
      Message::MouseEvent(event, id) => {
        if self.border_top.has(id) {
          return self
            .border_top
            .update(border::Message::MouseEvent(event))
            .map(|m| Message::Border(border::Position::Top, m));
        }

        if self.border_bottom.has(id) {
          return self
            .border_bottom
            .update(border::Message::MouseEvent(event))
            .map(|m| Message::Border(border::Position::Bottom, m));
        }

        Task::none()
      }

      Message::Border(position, message) => {
        let mut tasks = vec![];

        tasks.push(
          match position {
            border::Position::Top => self.border_top.update(message.clone()),
            border::Position::Bottom => self.border_bottom.update(message.clone()),
          }
          .map(move |m| Message::Border(position, m)),
        );

        match message {
          border::Message::Entered => match position {
            border::Position::Bottom => tasks.push(Task::done(Message::ShowClock)),
            border::Position::Top => {}
          },
          border::Message::Left => match position {
            border::Position::Bottom => {
              tasks.push(Task::done(Message::Clock(clock::Message::StartCloseTimer)))
            }
            border::Position::Top => {}
          },
          _ => {}
        }

        Task::batch(tasks)
      }

      Message::Clock(message) => {
        let mut tasks = vec![];
        if let Some(clock) = self.clock.as_mut() {
          tasks.push(clock.update(message).map(Message::Clock));
        }

        if message == clock::Message::Close {
          self.clock = None;
        }

        Task::batch(tasks)
      }

      Message::ShowClock => {
        if self.clock.is_some() {
          return Task::done(Message::Clock(clock::Message::StopCloseTimer));
        }

        let (clock, clock_task) = clock::Clock::new(&self.wl_output);
        self.clock = Some(clock);
        clock_task.map(Message::Clock)
      }

      Message::Ignore => Task::none(),
    }
  }

  pub fn view(&self, id: Id) -> Element<crate::Message> {
    if self.border_top.has(id) {
      return self.border_top.view();
    }

    if self.border_bottom.has(id) {
      return self.border_bottom.view();
    }

    if let Some(clock) = self.clock.as_ref() {
      return clock.view();
    }

    warn!("Window id not found in window elements");
    horizontal_space().into()
  }

  pub fn destroy(&self) -> Task<Message> {
    let mut result = vec![
      self.border_top.destroy().map(|_| Message::Ignore),
      self.border_bottom.destroy().map(|_| Message::Ignore),
    ];

    if let Some(clock) = self.clock.as_ref() {
      result.push(clock.destroy().map(|_| Message::Ignore));
    }

    Task::batch(result)
  }
}
