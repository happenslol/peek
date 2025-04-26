#![allow(dead_code)]
use std::collections::{HashMap, HashSet};

use anyhow::Result;
use iced::{
  Color, Element, Length, Task, Theme,
  daemon::Appearance,
  event::{
    listen_with,
    wayland::{Event as WaylandEvent, OutputEvent},
  },
  platform_specific::shell::commands::{
    layer_surface::{destroy_layer_surface, get_layer_surface},
    subsurface::{Anchor, KeyboardInteractivity, Layer},
  },
  runtime::platform_specific::wayland::layer_surface::{
    IcedMargin, IcedOutput, SctkLayerSurfaceSettings,
  },
  widget::{Space, container},
};
use tracing::{debug, info};
use tracing_subscriber::prelude::*;
use wayland_client::protocol::wl_output::WlOutput;

fn main() -> Result<()> {
  tracing_subscriber::registry()
    .with(tracing_subscriber::fmt::layer())
    .with(
      tracing_subscriber::filter::Targets::new()
        .with_target(env!("CARGO_PKG_NAME"), tracing::Level::DEBUG),
    )
    .init();

  iced::daemon(App::title, App::update, App::view)
    .style(App::style)
    .subscription(App::subscription)
    .run_with(App::new)?;

  Ok(())
}

#[derive(Debug, Clone)]
struct OutputWindows {
  border_top: iced::window::Id,
  border_bottom: iced::window::Id,
  clock: Option<iced::window::Id>,
  initial_entered_top: bool,
  initial_entered_bottom: bool,
}

impl OutputWindows {
  fn destroy(&self) -> Vec<iced::Task<Message>> {
    let mut result = vec![
      destroy_layer_surface(self.border_top),
      destroy_layer_surface(self.border_bottom),
    ];

    if let Some(clock) = self.clock {
      result.push(destroy_layer_surface(clock));
    }

    result
  }
}

#[derive(Debug, Clone, Default)]
struct App {
  windows: HashMap<WlOutput, OutputWindows>,
}

impl App {
  fn new() -> (Self, iced::Task<Message>) {
    (Self::default(), iced::Task::none())
  }

  fn title(&self, _id: iced::window::Id) -> String {
    String::from("panel")
  }

  fn subscription(&self) -> iced::Subscription<Message> {
    listen_with(|evt, _, id| match evt {
      iced::Event::PlatformSpecific(iced::event::PlatformSpecific::Wayland(
        WaylandEvent::Output(event, wl_output),
      )) => Some(Message::OutputEvent(event, wl_output)),

      ev => Some(Message::Event(ev, id)),
    })
  }

  fn update(&mut self, message: Message) -> iced::Task<Message> {
    match message {
      Message::Event(event, id) => {
        let Some((output, windows)) = self
          .windows
          .iter_mut()
          .find(|(_, window)| window.border_top == id || window.border_bottom == id)
        else {
          return Task::none();
        };

        match event {
          iced::Event::Mouse(iced::mouse::Event::CursorEntered) => {
            if id == windows.border_top && !windows.initial_entered_top {
              windows.initial_entered_top = true;
              return Task::none();
            }

            if id == windows.border_bottom && !windows.initial_entered_bottom {
              windows.initial_entered_bottom = true;
              return Task::none();
            }

            if windows.clock.is_some() {
              return Task::none();
            }

            Task::done(Message::ShowClock(output.clone()))
          }
          iced::Event::Mouse(iced::mouse::Event::CursorLeft) => Task::none(),
          _ => Task::none(),
        }
      }
      Message::ShowClock(output) => {
        println!("Show clock message received");

        if self.windows.get(&output).and_then(|w| w.clock).is_some() {
          println!("Clock already shown");
          return Task::none();
        }

        let clock_id = iced::window::Id::unique();
        let clock = get_layer_surface(SctkLayerSurfaceSettings {
          id: clock_id,
          layer: Layer::Overlay,
          keyboard_interactivity: KeyboardInteractivity::None,
          pointer_interactivity: false,
          anchor: Anchor::TOP,
          output: IcedOutput::Output(output.clone()),
          namespace: "panel".to_string(),
          margin: IcedMargin {
            top: 10,
            ..Default::default()
          },
          size: Some((Some(100), Some(100))),
          ..Default::default()
        });

        self.windows.get_mut(&output).unwrap().clock = Some(clock_id);

        clock
      }
      Message::OutputEvent(event, wl_output) => match event {
        iced::event::wayland::OutputEvent::Created(info) => {
          info!("Output created: {:?}", info);

          if self.windows.contains_key(&wl_output) {
            return Task::none();
          }

          let top_id = iced::window::Id::unique();
          let top = get_layer_surface(SctkLayerSurfaceSettings {
            id: top_id,
            layer: Layer::Overlay,
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            pointer_interactivity: true,
            anchor: Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
            output: IcedOutput::Output(wl_output.clone()),
            namespace: "panel".to_string(),
            size: Some((None, Some(10))),
            ..Default::default()
          });

          let bottom_id = iced::window::Id::unique();
          let bottom = get_layer_surface(SctkLayerSurfaceSettings {
            id: bottom_id,
            layer: Layer::Overlay,
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            pointer_interactivity: true,
            anchor: Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
            output: IcedOutput::Output(wl_output.clone()),
            namespace: "panel".to_string(),
            size: Some((None, Some(10))),
            ..Default::default()
          });

          self.windows.insert(
            wl_output,
            OutputWindows {
              border_top: top_id,
              border_bottom: bottom_id,
              clock: None,
              initial_entered_top: false,
              initial_entered_bottom: false,
            },
          );

          Task::batch(vec![top, bottom])
        }
        iced::event::wayland::OutputEvent::Removed => {
          if let Some(windows) = self.windows.remove(&wl_output) {
            return Task::batch(windows.destroy());
          }

          Task::none()
        }
        _ => Task::none(),
      },
      _ => Task::none(),
    }
  }

  pub fn view(&self, id: iced::window::Id) -> Element<Message> {
    for windows in self.windows.values() {
      if windows.border_top == id || windows.border_bottom == id {
        return Space::new(0, 0).into();
      }

      if windows.clock == Some(id) {
        return container(Space::new(Length::Fill, Length::Fill))
          .style(|_theme| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0., 0., 0.))),
            ..Default::default()
          })
          .into();
      }
    }

    Space::new(Length::Fill, Length::Fill).into()
  }

  pub fn style(&self, theme: &Theme) -> Appearance {
    Appearance {
      background_color: Color::TRANSPARENT,
      text_color: theme.palette().text,
      icon_color: theme.palette().text,
    }
  }
}

#[derive(Debug, Clone, Copy)]
enum Border {
  Top,
  Bottom,
}

#[derive(Debug, Clone)]
enum Message {
  CreateCorners,
  ShowClock(WlOutput),
  OutputEvent(OutputEvent, WlOutput),
  Event(iced::Event, iced::window::Id),
}
