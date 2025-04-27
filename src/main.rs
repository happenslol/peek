use std::collections::HashMap;

use iced::{
  Color, Element, Length, Subscription, Task, Theme,
  daemon::Appearance,
  event::{
    listen_with,
    wayland::{Event as WaylandEvent, OutputEvent},
  },
  widget::Space,
  window::Id,
};
use output::Output;
use tracing::{info, level_filters::LevelFilter, warn};
use wayland_client::protocol::wl_output::WlOutput;

mod border;
mod clock;
mod output;

fn main() -> iced::Result {
  let filter = tracing_subscriber::EnvFilter::try_from_env("PEEK_LOG").unwrap_or_else(|_| {
    tracing_subscriber::EnvFilter::default()
      .add_directive(format!("{}=info", env!("CARGO_PKG_NAME")).parse().unwrap())
      .add_directive(LevelFilter::WARN.into())
  });

  tracing_subscriber::fmt().with_env_filter(filter).init();

  iced::daemon(App::title, App::update, App::view)
    .style(App::style)
    .theme(App::theme)
    .subscription(App::subscription)
    .run_with(App::new)
}

#[derive(Debug, Clone, Default)]
struct App {
  outputs: HashMap<WlOutput, Output>,
}

impl App {
  fn new() -> (Self, iced::Task<Message>) {
    (Self::default(), iced::Task::none())
  }

  fn title(&self, _id: Id) -> String {
    String::from("panel")
  }

  fn theme(&self, _id: Id) -> Theme {
    Theme::Dark
  }

  pub fn style(&self, theme: &Theme) -> Appearance {
    Appearance {
      background_color: Color::TRANSPARENT,
      text_color: theme.palette().text,
      icon_color: theme.palette().text,
    }
  }

  fn subscription(&self) -> iced::Subscription<Message> {
    let mut result = vec![listen_with(|event, _, id| match event {
      iced::Event::PlatformSpecific(iced::event::PlatformSpecific::Wayland(
        WaylandEvent::Output(event, wl_output),
      )) => Some(Message::WaylandOutputEvent(event, wl_output)),

      iced::Event::Mouse(event) => Some(Message::MouseEvent(event, id)),
      _ => None,
    })];

    result.extend(self.outputs.iter().filter_map(|(wl_output, output)| {
      let message = output
        .subscription()?
        .with(wl_output.clone())
        .map(|(o, m)| Message::Output(o, m));

      Some(message)
    }));

    Subscription::batch(result)
  }

  fn update(&mut self, message: Message) -> iced::Task<Message> {
    match message {
      Message::MouseEvent(event, id) => {
        let Some((wl_output, output)) = self.outputs.iter_mut().find(|(_, w)| w.has(id)) else {
          warn!("Window id {id} not found in window elements");
          return Task::none();
        };

        let wl_output = wl_output.clone();
        output
          .update(output::Message::MouseEvent(event, id))
          .map(move |m| Message::Output(wl_output.clone(), m))
      }
      Message::WaylandOutputEvent(event, wl_output) => match event {
        iced::event::wayland::OutputEvent::Created(info) => {
          info!("Output created: {:?}", info);

          if self.outputs.contains_key(&wl_output) {
            return Task::none();
          }

          let (output, task) = Output::new(&wl_output);
          self.outputs.insert(wl_output.clone(), output);

          task.map(move |m| Message::Output(wl_output.clone(), m))
        }
        iced::event::wayland::OutputEvent::Removed => self
          .outputs
          .remove(&wl_output)
          .map_or_else(Task::none, |o| o.destroy().map(|_| Message::Ignore)),
        _ => Task::none(),
      },

      Message::Output(wl_output, message) => {
        self
          .outputs
          .get_mut(&wl_output)
          .map_or_else(Task::none, |o| {
            o.update(message)
              .map(move |m| Message::Output(wl_output.clone(), m))
          })
      }

      Message::Ignore => Task::none(),
    }
  }

  pub fn view(&self, id: Id) -> Element<Message> {
    for output in self.outputs.values() {
      if output.has(id) {
        return output.view(id);
      }
    }

    Space::new(Length::Fill, Length::Fill).into()
  }
}

#[derive(Debug, Clone)]
enum Message {
  Ignore,
  Output(WlOutput, output::Message),
  WaylandOutputEvent(OutputEvent, WlOutput),
  MouseEvent(iced::mouse::Event, Id),
}
