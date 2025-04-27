use std::time::Duration;

use chrono::{DateTime, Local};
use futures::future::{AbortHandle, Aborted, abortable};
use iced::{
  Alignment, Color, Element, Length, Subscription, Task, border,
  platform_specific::shell::commands::{
    layer_surface::{destroy_layer_surface, get_layer_surface},
    subsurface::{Anchor, KeyboardInteractivity, Layer},
  },
  runtime::platform_specific::wayland::layer_surface::{
    IcedMargin, IcedOutput, SctkLayerSurfaceSettings,
  },
  time::every,
  widget::{column, container, text},
  window::Id,
};
use wayland_client::protocol::wl_output::WlOutput;

#[derive(Debug, Clone)]
pub struct Clock {
  id: Id,
  date: DateTime<Local>,
  close_timer_abort: Option<AbortHandle>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Message {
  Ignore,
  StartCloseTimer,
  StopCloseTimer,
  Close,
  Update,
}

impl Clock {
  pub fn new(output: &WlOutput) -> (Self, Task<Message>) {
    let id = Id::unique();

    let task = get_layer_surface(SctkLayerSurfaceSettings {
      id,
      layer: Layer::Overlay,
      keyboard_interactivity: KeyboardInteractivity::None,
      pointer_interactivity: false,
      anchor: Anchor::BOTTOM,
      output: IcedOutput::Output(output.clone()),
      namespace: "panel".to_string(),
      margin: IcedMargin {
        bottom: 10,
        ..Default::default()
      },
      size: Some((Some(200), Some(120))),
      ..Default::default()
    });

    let result = Self {
      id,
      date: Local::now(),
      close_timer_abort: None,
    };

    (result, task)
  }

  pub fn has(&self, id: Id) -> bool {
    self.id == id
  }

  pub fn subscription(&self) -> Subscription<Message> {
    every(Duration::from_secs(5)).map(|_| Message::Update)
  }

  pub fn update(&mut self, message: Message) -> Task<Message> {
    match message {
      Message::Update => {
        self.date = Local::now();
        Task::none()
      }
      Message::Close => self.destroy(),
      Message::StartCloseTimer => {
        let (task, timer_abort) = close_timer();
        self.close_timer_abort = Some(timer_abort);
        task
      }
      Message::StopCloseTimer => {
        if let Some(timer_abort) = self.close_timer_abort.take() {
          timer_abort.abort();
        }

        Task::none()
      }
      Message::Ignore => Task::none(),
    }
  }

  pub fn view(&self) -> Element<crate::Message> {
    container(
      container(
        column![
          text(self.date.format("%H:%M").to_string()).size(38),
          text(self.date.format("%a, %e %b %G").to_string()).size(14)
        ]
        .align_x(Alignment::Center),
      )
      .center(Length::Fill)
      .style(|_theme| container::Style {
        background: Some(iced::Background::Color(Color::from_rgb(0.1, 0.1, 0.1))),
        border: border::rounded(16)
          .color(Color::from_rgb(0.5, 0.5, 0.5))
          .width(2.0),
        ..Default::default()
      }),
    )
    .padding(2)
    .into()
  }

  pub fn destroy(&self) -> iced::Task<Message> {
    destroy_layer_surface(self.id)
  }
}

fn close_timer() -> (Task<Message>, AbortHandle) {
  let (future, timer_abort) = abortable(async {
    let duration = Duration::from_millis(200);
    tokio::time::sleep(duration).await;
  });

  let task = Task::perform(future, |result| match result {
    Err(Aborted) => Message::Ignore,
    _ => Message::Close,
  });

  (task, timer_abort)
}
