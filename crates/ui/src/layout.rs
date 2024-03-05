#[derive(Debug)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[derive(Debug)]
pub struct Rectangle {
    color: Color,
}

#[derive(Debug)]
pub struct TexturedRectangle {
    texture: usize, // some kind of id into a texture atlas
}

#[derive(Debug)]
pub enum Ui {
    Hbox(Hbox),
    Vbox(Vbox),
    Rectangle(Rectangle),
    TexturedRectangle(TexturedRectangle),
}

#[derive(Debug)]
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
        ((self.min.0 + self.max.0) / 2.0, (self.min.1 + self.max.1) / 2.0)
    }

    pub fn top_left(&self) -> (f32, f32) {
        (self.min.0, self.max.1)
    }

    // Returns true if pos is inside the bbox.
    pub fn inside(&self, pos: (f32, f32)) -> bool {
        let x_inside = self.min.0 <= pos.0 && pos.0 <= self.max.0;
        let y_inside = self.min.1 <= pos.1 && pos.1 <= self.max.1;

        x_inside && y_inside
    }
}

pub trait UiElement {
    fn layout(&self, parent_size: BoundingBox, rects: &mut Vec<BoundingBox>) {
        for (i, elem) in self.elements().iter().enumerate() {
            let child_bbox = if self.is_hbox() {
                let child_index = i;
                let child_width = parent_size.width() / self.elements().len() as f32;
                let x0 = parent_size.min.0 + child_width * child_index as f32;

                BoundingBox::new(x0, parent_size.min.1, x0 + child_width, parent_size.max.1)
            } else {
                let child_index = self.elements().len() - i - 1;
                let child_height = parent_size.height() / self.elements().len() as f32;
                let y0 = parent_size.min.1 + child_height * child_index as f32;

                BoundingBox::new(parent_size.min.0, y0, parent_size.max.0, y0 + child_height)
            };

            match elem {
                Ui::Rectangle(_) => rects.push(child_bbox),
                Ui::TexturedRectangle(_) => rects.push(child_bbox),
                Ui::Hbox(hbox) => hbox.layout(child_bbox, rects),
                Ui::Vbox(vbox) => vbox.layout(child_bbox, rects),
            }
        }
    }

    fn elements(&self) -> &Vec<Ui>;

    fn is_hbox(&self) -> bool;
}

#[derive(Debug)]
pub struct Hbox {
    elements: Vec<Ui>,
}

impl UiElement for Hbox {
    fn elements(&self) -> &Vec<Ui> {
        &self.elements
    }

    fn is_hbox(&self) -> bool {
        true
    }
}

#[derive(Debug)]
pub struct Vbox {
    elements: Vec<Ui>,
}

impl UiElement for Vbox {
    fn elements(&self) -> &Vec<Ui> {
        &self.elements
    }

    fn is_hbox(&self) -> bool {
        false
    }
}

#[test]
fn test_child() {
    let tree = Hbox {
        elements: vec![
            Ui::Rectangle(Rectangle {
                color: Color {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 255,
                },
            }),
            Ui::Rectangle(Rectangle {
                color: Color {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 255,
                },
            }),
            Ui::Rectangle(Rectangle {
                color: Color {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 255,
                },
            }),
        ],
    };

    // let expected_bounding_boxes = vec![];

    let parent_size = BoundingBox::new(0.0, 0.0, 800.0, 600.0);
    let mut rects = vec![];

    tree.layout(parent_size, &mut rects);

    println!("{:?}", rects);
}