use iced::{
  Element, Task,
  platform_specific::shell::commands::{
    layer_surface::{destroy_layer_surface, get_layer_surface},
    subsurface::{Anchor, KeyboardInteractivity, Layer},
  },
  runtime::platform_specific::wayland::layer_surface::{IcedOutput, SctkLayerSurfaceSettings},
  widget::horizontal_space,
  window::Id,
};
use tracing::debug;
use wayland_client::protocol::wl_output::WlOutput;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Position {
  Top,
  Bottom,
}

#[derive(Debug, Clone)]
pub struct Border {
  id: iced::window::Id,
  position: Position,
  initial_entered: bool,
  active: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
  Entered,
  Left,
  MouseEvent(iced::mouse::Event),
}

impl Border {
  pub fn new(output: &WlOutput, position: Position) -> (Self, Task<crate::Message>) {
    let id = Id::unique();

    let task = get_layer_surface(SctkLayerSurfaceSettings {
      id,
      layer: Layer::Overlay,
      keyboard_interactivity: KeyboardInteractivity::OnDemand,
      pointer_interactivity: true,
      anchor: match position {
        Position::Top => Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
        Position::Bottom => Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
      },
      output: IcedOutput::Output(output.clone()),
      namespace: "panel".to_string(),
      size: Some((None, Some(10))),
      ..Default::default()
    });

    let result = Self {
      id,
      position,
      initial_entered: false,
      active: false,
    };

    (result, task)
  }

  pub fn has(&self, id: iced::window::Id) -> bool {
    self.id == id
  }

  pub fn update(&mut self, message: Message) -> Task<Message> {
    match message {
      Message::MouseEvent(event) => match event {
        iced::mouse::Event::CursorEntered => {
          if !self.initial_entered {
            self.initial_entered = true;
            return Task::none();
          }

          self.active = true;
          debug!("Border entered: {:?}", self.position);
          Task::done(Message::Entered)
        }
        iced::mouse::Event::CursorLeft => {
          if !self.active {
            return Task::none();
          }

          debug!("Border left: {:?}", self.position);
          Task::done(Message::Left)
        }
        _ => Task::none(),
      },
      _ => Task::none(),
    }
  }

  pub fn view(&self) -> Element<crate::Message> {
    horizontal_space().into()
  }

  pub fn destroy(&self) -> iced::Task<crate::Message> {
    destroy_layer_surface(self.id)
  }
}
