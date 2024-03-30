use crate::{
    image_pipeline::{self, ImageInstance},
    quad_pipeline::QuadInstance,
    texture_atlas::{TextureAtlas, TextureId},
};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};
use text_editor::{ScrollAmount, TextEditor};
use winit::{
    event::{ElementState, KeyEvent, MouseScrollDelta},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

#[derive(Debug, Clone, Copy)]
pub struct UiNodeId(usize);

#[derive(Debug)]
pub enum Drawables {
    Rect(QuadInstance),
    TexturedRect(ImageInstance),
}

#[derive(Debug, Clone, Copy)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn to_f32_arr(&self) -> [f32; 4] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        ]
    }
}

#[derive(Debug)]
pub struct Rectangle {
    color: Color,
}

impl Rectangle {
    fn layout(&self, view_size: BoundingBox, drawables: &mut Vec<Drawables>) {
        drawables.push(Drawables::Rect(QuadInstance {
            position: [view_size.min.0, view_size.min.1],
            size: [view_size.width(), view_size.height()],
            color: self.color.to_f32_arr(),
        }));
    }
}

#[derive(Debug)]
pub struct TexturedRectangle {
    texture_id: TextureId,
    tint: Color,
}

impl TexturedRectangle {
    fn layout(
        &self,
        atlas: &mut TextureAtlas,
        view_size: BoundingBox,
        drawables: &mut Vec<Drawables>,
    ) {
        drawables.push(Drawables::TexturedRect(ImageInstance::add_instance(
            atlas,
            self.texture_id,
            [view_size.min.0, view_size.min.1],
            [view_size.width(), view_size.height()],
            self.tint.to_f32_arr(),
        )));
    }
}

#[derive(Debug)]
pub struct FixedSizedBox {
    width: f32,
    height: f32,
    child: UiNodeId,
    background_color: Color,
}

impl FixedSizedBox {
    fn layout(
        &self,
        scene: &Scene,
        atlas: &mut TextureAtlas,
        view_size: BoundingBox,
        queue: &wgpu::Queue,
        window: &Window,
        drawables: &mut Vec<Drawables>,
    ) {
        drawables.push(Drawables::Rect(QuadInstance {
            position: [view_size.min.0, view_size.min.1],
            size: [view_size.width(), view_size.height()],
            color: self.background_color.to_f32_arr(),
        }));

        // The ceneter of the space we have
        let bbox_center = view_size.center();
        let half_width = self.width / 2.0;
        let half_height = self.height / 2.0;

        let fixed_size_bbox = BoundingBox::new(
            bbox_center.0 - half_width,
            bbox_center.1 - half_height,
            bbox_center.0 + half_width,
            bbox_center.1 + half_height,
        );

        let child = scene.node(self.child);
        child.layout(scene, atlas, fixed_size_bbox, queue, window, drawables);
    }
}

#[derive(Debug)]
pub struct Text {
    /// Contains all of the text within this text editor.
    editor: TextEditor,

    font_size: f32,
    text_color: Color,
    background_color: Color,

    /// The last time something was entered in the text editor. Used to see if
    /// we should keep the cursor visible or allow it to blink.
    last_action: Instant,

    /// The last time the cursor blinked. Used to alternate drawing the cursor
    /// and create the blinking effect.
    last_cursor_blink: Instant,
}

impl Text {
    fn layout(
        &mut self,
        atlas: &mut TextureAtlas,
        view_size: BoundingBox,
        queue: &wgpu::Queue,
        drawables: &mut Vec<Drawables>,
    ) {
        // background color
        drawables.push(Drawables::Rect(QuadInstance {
            position: [view_size.min.0, view_size.min.1],
            size: [view_size.width(), view_size.height()],
            color: self.background_color.to_f32_arr(),
        }));

        // Default cursor blink rate is 530ms. TIL
        // Only blink cursor if there was no action in the last second
        let draw_cursor = if Instant::now().duration_since(self.last_action)
            > Duration::from_millis(1060)
        {
            if Instant::now().duration_since(self.last_cursor_blink) > Duration::from_millis(530) {
                if Instant::now().duration_since(self.last_cursor_blink)
                    > Duration::from_millis(1060)
                {
                    self.last_cursor_blink = Instant::now();
                }
                true
            } else {
                false
            }
        } else {
            true
        };

        drawables.extend(image_pipeline::layout_text(
            view_size,
            atlas,
            self.font_size,
            queue,
            &self.text_color,
            draw_cursor,
            &self.editor,
        ));
    }

    pub fn backspace(&mut self) {
        self.editor.backspace();
    }

    pub fn delete(&mut self) {
        todo!()
        // self.text
        //     .borrow_mut()
        //     .replace_range(self.cursor_position..self.cursor_position + 1, "");
    }

    pub fn add_char(&mut self, c: &str) {
        self.editor.add_char(c);
    }

    /// Scrolls the text viewport 'scroll_lines' at a time.
    pub fn scroll(&mut self, delta: MouseScrollDelta, lines: usize) {
        let scroll_amount = match delta {
            MouseScrollDelta::LineDelta(_, y) => {
                if y > 0.0 {
                    ScrollAmount::Up { lines }
                } else {
                    ScrollAmount::Down { lines }
                }
            }
            MouseScrollDelta::PixelDelta(_) => todo!(),
        };

        self.editor.scroll(scroll_amount);
    }
}

#[derive(Debug)]
pub struct Hbox {
    elements: Vec<UiNodeId>,
}

impl Hbox {
    fn layout(
        &self,
        scene: &Scene,
        atlas: &mut TextureAtlas,
        parent_size: BoundingBox,
        queue: &wgpu::Queue,
        window: &Window,
        drawables: &mut Vec<Drawables>,
    ) {
        for (i, id) in self.elements.iter().enumerate() {
            let child_index = i;
            let child_width = parent_size.width() / self.elements.len() as f32;
            let x0 = parent_size.min.0 + child_width * child_index as f32;

            let view_size =
                BoundingBox::new(x0, parent_size.min.1, x0 + child_width, parent_size.max.1);

            scene
                .node(*id)
                .layout(scene, atlas, view_size, queue, window, drawables);
        }
    }
}

#[derive(Debug)]
pub struct Vbox {
    elements: Vec<UiNodeId>,
}

impl Vbox {
    fn layout(
        &self,
        scene: &Scene,
        atlas: &mut TextureAtlas,
        parent_size: BoundingBox,
        queue: &wgpu::Queue,
        window: &Window,
        drawables: &mut Vec<Drawables>,
    ) {
        for (i, id) in self.elements.iter().enumerate() {
            let child_index = self.elements.len() - i - 1;
            let child_height = parent_size.height() / self.elements.len() as f32;
            let y0 = parent_size.min.1 + child_height * child_index as f32;

            let view_size =
                BoundingBox::new(parent_size.min.0, y0, parent_size.max.0, y0 + child_height);

            scene
                .node(*id)
                .layout(scene, atlas, view_size, queue, window, drawables);
        }
    }
}

#[derive(Debug)]
pub enum Ui {
    TexturedRectangle(TexturedRectangle),
    FixedSizedBox(FixedSizedBox),
    Rectangle(Rectangle),
    Text(RefCell<Text>),
    Hbox(Hbox),
    Vbox(Vbox),
    Spacer,
}

impl Ui {
    fn layout(
        &self,
        scene: &Scene,
        atlas: &mut TextureAtlas,
        view_size: BoundingBox,
        queue: &wgpu::Queue,
        window: &Window,
        drawables: &mut Vec<Drawables>,
    ) {
        match self {
            Ui::TexturedRectangle(tr) => tr.layout(atlas, view_size, drawables),
            Ui::FixedSizedBox(fsb) => fsb.layout(scene, atlas, view_size, queue, window, drawables),
            Ui::Rectangle(r) => r.layout(view_size, drawables),
            Ui::Text(td) => td.borrow_mut().layout(atlas, view_size, queue, drawables),
            Ui::Hbox(h) => h.layout(scene, atlas, view_size, queue, window, drawables),
            Ui::Vbox(v) => v.layout(scene, atlas, view_size, queue, window, drawables),
            Ui::Spacer => {}
        }
    }
}

#[derive(Debug)]
pub struct Scene {
    nodes: RefCell<Vec<Rc<Ui>>>,
    node_root: UiNodeId,
    cursor_pos: (f32, f32),
    focused: Option<UiNodeId>,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            nodes: RefCell::new(vec![]),
            node_root: UiNodeId(0),
            cursor_pos: (0.0, 0.0),
            focused: None,
        }
    }
}

impl Scene {
    pub fn set_focus(&mut self, node: UiNodeId) {
        self.focused = Some(node);
    }

    pub fn set_root(&mut self, root: UiNodeId) {
        self.node_root = root;
    }

    pub fn scroll(&self, delta: MouseScrollDelta) {
        if let Some(focused) = self.focused {
            if let Ui::Text(td) = self.node(focused).as_ref() {
                td.borrow_mut().scroll(delta, 2);
            }
        }
    }

    pub fn send_keystroke(&mut self, event: &KeyEvent) {
        if let Some(focused) = self.focused {
            if let Ui::Text(td) = self.node(focused).as_ref() {
                let mut td = td.borrow_mut();
                if event.state == ElementState::Pressed {
                    match event.physical_key {
                        PhysicalKey::Code(c) => {
                            let mut other_action = false;

                            let ch = match c {
                                KeyCode::KeyA => "a",
                                KeyCode::KeyB => "b",
                                KeyCode::KeyC => "c",
                                KeyCode::KeyD => "d",
                                KeyCode::KeyE => "e",
                                KeyCode::KeyF => "f",
                                KeyCode::KeyG => "g",
                                KeyCode::KeyH => "h",
                                KeyCode::KeyI => "i",
                                KeyCode::KeyJ => "j",
                                KeyCode::KeyK => "k",
                                KeyCode::KeyL => "l",
                                KeyCode::KeyM => "m",
                                KeyCode::KeyN => "n",
                                KeyCode::KeyO => "o",
                                KeyCode::KeyP => "p",
                                KeyCode::KeyQ => "q",
                                KeyCode::KeyR => "r",
                                KeyCode::KeyS => "s",
                                KeyCode::KeyT => "t",
                                KeyCode::KeyU => "u",
                                KeyCode::KeyV => "v",
                                KeyCode::KeyW => "w",
                                KeyCode::KeyX => "x",
                                KeyCode::KeyY => "y",
                                KeyCode::KeyZ => "z",
                                KeyCode::Space => " ",
                                KeyCode::Enter => "\n",
                                KeyCode::Backquote => "`",
                                KeyCode::Backslash => "\\",
                                KeyCode::BracketLeft => "[",
                                KeyCode::BracketRight => "]",
                                KeyCode::Comma => ",",
                                KeyCode::Digit0 => "0",
                                KeyCode::Digit1 => "1",
                                KeyCode::Digit2 => "2",
                                KeyCode::Digit3 => "3",
                                KeyCode::Digit4 => "4",
                                KeyCode::Digit5 => "5",
                                KeyCode::Digit6 => "6",
                                KeyCode::Digit7 => "7",
                                KeyCode::Digit8 => "8",
                                KeyCode::Digit9 => "9",
                                KeyCode::Equal => "=",
                                KeyCode::Minus => "-",
                                KeyCode::Period => ".",
                                KeyCode::Quote => "\"",
                                KeyCode::Semicolon => ";",
                                KeyCode::Slash => "/",
                                KeyCode::Tab => "\t",
                                KeyCode::Delete => {
                                    other_action = true;
                                    td.delete();
                                    " "
                                }
                                KeyCode::Backspace => {
                                    other_action = true;
                                    td.backspace();
                                    " "
                                }
                                // KeyCode::ArrowLeft => {
                                //     td.cursor_position -= 1;
                                //     " "
                                // }
                                // KeyCode::ArrowRight => {
                                //     if td.cursor_position < td.text.borrow().len() {
                                //         td.cursor_position += 1;
                                //     }
                                //     " "
                                // }
                                _ => return,
                            };

                            td.last_action = Instant::now();
                            if !other_action {
                                td.add_char(ch);
                            }
                        }
                        _ => todo!(),
                    }
                }
            }
        }
    }

    pub fn layout(
        &self,
        atlas: &mut TextureAtlas,
        view_size: (f32, f32),
        queue: &wgpu::Queue,
        window: &Window,
    ) -> Vec<Drawables> {
        let parent_size = BoundingBox {
            min: (0.0, 0.0),
            max: (view_size.0, view_size.1),
        };

        let mut drawables = vec![];

        self.node(self.node_root)
            .layout(self, atlas, parent_size, queue, window, &mut drawables);

        drawables
    }

    pub fn fixed_size_bbox(
        &self,
        width: f32,
        height: f32,
        child: UiNodeId,
        background_color: Color,
    ) -> UiNodeId {
        let obj = FixedSizedBox {
            width,
            height,
            child,
            background_color,
        };
        let idx = self.nodes.borrow().len();
        self.nodes
            .borrow_mut()
            .push(Rc::new(Ui::FixedSizedBox(obj)));
        UiNodeId(idx)
    }

    pub fn textured_rectangle(&self, texture_id: TextureId) -> UiNodeId {
        let obj = TexturedRectangle {
            texture_id,
            tint: Color::new(255, 255, 255, 255),
        };
        let idx = self.nodes.borrow().len();
        self.nodes
            .borrow_mut()
            .push(Rc::new(Ui::TexturedRectangle(obj)));
        UiNodeId(idx)
    }

    pub fn textured_rectangle_tinted(&self, texture_id: TextureId, tint: Color) -> UiNodeId {
        let obj = TexturedRectangle { texture_id, tint };
        let idx = self.nodes.borrow().len();
        self.nodes
            .borrow_mut()
            .push(Rc::new(Ui::TexturedRectangle(obj)));
        UiNodeId(idx)
    }

    pub fn rectangle(&self, color: Color) -> UiNodeId {
        let obj = Rectangle { color };
        let idx = self.nodes.borrow().len();
        self.nodes.borrow_mut().push(Rc::new(Ui::Rectangle(obj)));
        UiNodeId(idx)
    }

    pub fn text_details(
        &self,
        text: String,
        font_size: f32,
        text_color: Color,
        background_color: Color,
    ) -> UiNodeId {
        // TODO: come up with a good default for word wrap here.
        let obj = Text {
            editor: TextEditor::new(&text, 80),
            font_size,
            text_color,
            background_color,
            last_cursor_blink: Instant::now(),
            last_action: Instant::now(),
        };
        let idx = self.nodes.borrow().len();
        self.nodes
            .borrow_mut()
            .push(Rc::new(Ui::Text(RefCell::new(obj))));
        UiNodeId(idx)
    }

    pub fn hbox(&self, elements: Vec<UiNodeId>) -> UiNodeId {
        let obj = Hbox { elements };
        let idx = self.nodes.borrow().len();
        self.nodes.borrow_mut().push(Rc::new(Ui::Hbox(obj)));
        UiNodeId(idx)
    }

    pub fn vbox(&self, elements: Vec<UiNodeId>) -> UiNodeId {
        let obj = Vbox { elements };
        let idx = self.nodes.borrow().len();
        self.nodes.borrow_mut().push(Rc::new(Ui::Vbox(obj)));
        UiNodeId(idx)
    }

    pub fn update_cursor_pos(&mut self, cx: f32, cy: f32) {
        self.cursor_pos = (cx, cy);
    }

    fn node(&self, id: UiNodeId) -> Rc<Ui> {
        self.nodes.borrow()[id.0].clone()
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct BoundingBox {
    pub min: (f32, f32),
    pub max: (f32, f32),
}

impl BoundingBox {
    pub fn new(x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        Self {
            min: (x0, y0),
            max: (x1, y1),
        }
    }

    pub fn width(&self) -> f32 {
        self.max.0 - self.min.0
    }

    pub fn height(&self) -> f32 {
        self.max.1 - self.min.1
    }

    pub fn center(&self) -> (f32, f32) {
        (
            (self.min.0 + self.max.0) / 2.0,
            (self.min.1 + self.max.1) / 2.0,
        )
    }

    pub fn top_left(&self) -> (f32, f32) {
        (self.min.0, self.min.1)
    }

    // Returns true if pos is inside the bbox.
    pub fn inside(&self, pos: (f32, f32)) -> bool {
        let x_inside = self.min.0 <= pos.0 && pos.0 <= self.max.0;
        let y_inside = self.min.1 <= pos.1 && pos.1 <= self.max.1;

        x_inside && y_inside
    }
}
